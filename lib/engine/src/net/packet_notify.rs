use std::{collections::VecDeque, fmt, io};

use bitstream_io::{BitRead, BitWrite};

type SequenceNumber = u16;
type Word = u32;
const SEQUENCE_NUMBER_BITS: u16 = 14;
const HISTORY_SIZE: u32 = 256;
const BITS_PER_WORD: u32 = Word::BITS;
const WORD_COUNT: u32 = HISTORY_SIZE / BITS_PER_WORD;

#[derive(Default)]
pub struct FNetPacketNotify {
    ack_record: VecDeque<FSentAckData>,
    written_history_word_count: usize,
    written_in_ack_seq: SequenceNumber,
    in_seq_history: SequenceHistory,
    in_seq: SequenceNumber,
    in_ack_seq: SequenceNumber,
    in_ack_seq_ack: SequenceNumber,
    out_seq: SequenceNumber,
    out_ack_seq: SequenceNumber,
    waiting_for_flush_seq_ack: SequenceNumber,
}

#[derive(Default)]
struct FSentAckData {
    out_seq: SequenceNumber,
    in_ack_seq: SequenceNumber,
}

#[derive(Default, Debug)]
pub struct SequenceHistory([Word; WORD_COUNT as usize]);

#[derive(Debug)]
pub struct FNotificationHeader {
    pub packed_header: FPackedHeader,
    pub _history: SequenceHistory,
}

#[derive(Debug, Clone, Copy)]
pub struct FPackedHeader(pub u32);

impl SequenceHistory {
    pub const SIZE: u32 = HISTORY_SIZE;
}

fn diff(a: SequenceNumber, b: SequenceNumber) -> u32 {
    ((a as i32) - (b as i32)).unsigned_abs()
}

impl FNetPacketNotify {
    pub fn init(&mut self, initial_in_seq: SequenceNumber, initial_out_seq: SequenceNumber) {
        self.in_seq = initial_in_seq;
        self.in_ack_seq = initial_in_seq;
        self.in_ack_seq_ack = initial_in_seq;
        self.out_seq = initial_out_seq;
        self.out_ack_seq = initial_out_seq - 1;
        self.waiting_for_flush_seq_ack = initial_out_seq - 1;
    }

    pub fn will_sequence_fit_in_sequence_history(&self, seq: SequenceNumber) -> bool {
        if seq >= self.in_ack_seq_ack {
            diff(seq, self.in_ack_seq_ack) <= SequenceHistory::SIZE
        } else {
            false
        }
    }

    pub fn set_wait_for_sequence_history_flush(&mut self) {
        self.waiting_for_flush_seq_ack = self.out_seq;
    }

    pub fn get_current_sequence_history_length(&self) -> u32 {
        if self.in_ack_seq >= self.in_ack_seq_ack {
            std::cmp::min(
                diff(self.in_ack_seq, self.in_ack_seq_ack),
                SequenceHistory::SIZE,
            )
        } else {
            SequenceHistory::SIZE
        }
    }

    pub fn is_waiting_for_sequence_history_flush(&self) -> bool {
        self.waiting_for_flush_seq_ack > self.out_ack_seq
    }

    pub fn process_received_acks(&mut self, notification_data: &FNotificationHeader) {
        if notification_data.packed_header.get_acked_seq() > self.out_ack_seq {
            let mut ack_count = diff(
                notification_data.packed_header.get_acked_seq(),
                self.out_ack_seq,
            );

            let new_in_ack_seq_ack = self
                .update_in_ack_seq_ack(ack_count, notification_data.packed_header.get_acked_seq());
            if new_in_ack_seq_ack > self.in_ack_seq_ack {
                self.in_ack_seq_ack = new_in_ack_seq_ack;
            }

            let mut current_ack = self.out_ack_seq;
            current_ack = current_ack.wrapping_add(1);

            while ack_count
                > notification_data.packed_header.get_history_word_count() as u32 * BITS_PER_WORD
            {
                ack_count -= 1;
                // in_func(current_ack, false)
                current_ack = current_ack.wrapping_add(1);
            }

            while ack_count > 0 {
                ack_count -= 1;
                // in_func(current_ack, notification_data.history.is_delivered);
                current_ack += 1;
            }

            self.out_ack_seq = notification_data.packed_header.get_acked_seq();

            if self.out_ack_seq > self.waiting_for_flush_seq_ack {
                self.waiting_for_flush_seq_ack = self.out_ack_seq;
            }
        }
    }

    pub fn internal_update(
        &mut self,
        notification_data: &FNotificationHeader,
        in_seq_delta: u32,
    ) -> u32 {
        if !self.is_waiting_for_sequence_history_flush()
            && !self
                .will_sequence_fit_in_sequence_history(notification_data.packed_header.get_seq())
        {
            if self.get_has_unacknowledged_acks() {
                self.set_wait_for_sequence_history_flush();
            } else {
                self.in_ack_seq_ack = notification_data.packed_header.get_seq() - 1;
            }
        }

        //  if !self.is_waiting_for_sequence_history_flush() {
        self.in_seq = notification_data.packed_header.get_seq();
        in_seq_delta
        //   } else {
        //       let mut new_in_seq_to_ack = notification_data.packed_header.get_seq();
        //       if !self.will_sequence_fit_in_sequence_history(new_in_seq_to_ack)
        //           && self.get_has_unacknowledged_acks()
        //       {
        //           new_in_seq_to_ack =
        //               self.in_ack_seq_ack + (256 - self.get_current_sequence_history_length()) as u16
        //       }

        //       if new_in_seq_to_ack >= self.in_seq {
        //           let adjusted_sequence_delta = diff(new_in_seq_to_ack, self.in_seq);
        //           self.in_seq = new_in_seq_to_ack;
        //           self.ack_seq(new_in_seq_to_ack, false);

        //           debug!("FNetPacketNotify::update - waiting for sequence history flush - rejected: {} accepted: in_seq: {} adjusted delta {}", notification_data.packed_header.get_seq(), self.in_seq, adjusted_sequence_delta);
        //           adjusted_sequence_delta
        //       } else {
        //           debug!("FNetPacketNotify::update - waiting for sequence history flush - rejected: {} accepted: in_seq: {}", notification_data.packed_header.get_seq(), self.in_seq);
        //           0
        //       }
        //   }
    }

    pub fn ack_seq(&mut self, acked_seq: SequenceNumber, is_ack: bool) {
        while acked_seq > self.in_ack_seq {
            self.in_ack_seq += 1;

            let report_acked = self.in_ack_seq == acked_seq && is_ack;
            // debug!(
            //     "FNetPacketNotify::ack_seq - acked_seq: {}, is_ack: {}, ack_history_size: {}",
            //     self.in_ack_seq,
            //     report_acked as u8,
            //     self.get_current_sequence_history_length()
            // );

            self.in_seq_history.add_delivery_status(report_acked);
        }
    }

    pub fn update_in_ack_seq_ack(
        &mut self,
        ack_count: u32,
        acked_seq: SequenceNumber,
    ) -> SequenceNumber {
        if (ack_count as usize) <= self.ack_record.len() {
            if ack_count > 1 {
                self.ack_record.pop_back();
            }

            let ack_data = self.ack_record.pop_front().unwrap_or_default();
            if ack_data.out_seq == acked_seq {
                return ack_data.in_ack_seq;
            }
        }

        // TODO: find out why this happens sometimes (usually on keep alives?)
        // acked_seq.wrapping_sub(256) // original code returns this

        acked_seq
    }

    pub fn get_has_unacknowledged_acks(&self) -> bool {
        for i in 0..self.get_current_sequence_history_length() {
            if self.in_seq_history.is_delivered(i as usize) {
                return true;
            }
        }

        false
    }

    pub fn write_header<W: BitWrite>(&mut self, w: &mut W, refresh: bool) -> io::Result<bool> {
        let current_history_word_count = self
            .get_current_sequence_history_length()
            .div_ceil(BITS_PER_WORD)
            .clamp(1, WORD_COUNT);

        if refresh && current_history_word_count as usize > self.written_history_word_count {
            return Ok(false);
        }

        self.written_history_word_count = if refresh {
            self.written_history_word_count
        } else {
            current_history_word_count as usize
        };

        let seq = self.out_seq;
        let acked_seq = self.in_ack_seq;

        let packed_header =
            FPackedHeader::pack(seq, acked_seq, self.written_history_word_count as u16 - 1);
        w.write(32, packed_header.0)?;
        self.in_seq_history
            .write(w, self.written_history_word_count)?;

        Ok(true)
    }

    pub fn read_header<R: BitRead>(&self, r: &mut R) -> io::Result<FNotificationHeader> {
        FNotificationHeader::read_header(r)
    }

    pub fn get_sequence_delta(&self, notification_data: &FNotificationHeader) -> u32 {
        if notification_data.packed_header.get_seq() > self.in_seq
            && notification_data.packed_header.get_acked_seq() >= self.out_ack_seq
            && self.out_seq >= notification_data.packed_header.get_acked_seq()
        {
            diff(notification_data.packed_header.get_seq(), self.in_seq)
        } else {
            0
        }
    }

    pub fn commit_and_increment_seq(&mut self) -> SequenceNumber {
        self.ack_record.push_back(FSentAckData {
            out_seq: self.out_seq,
            in_ack_seq: self.written_in_ack_seq,
        });
        self.written_history_word_count = 0;

        self.out_seq += 1;
        self.out_seq
    }
}

impl SequenceHistory {
    pub fn add_delivery_status(&mut self, delivered: bool) {
        const VALUE_MASK: Word = 1 << (BITS_PER_WORD - 1);
        let mut carry = delivered as Word;

        for i in 0..WORD_COUNT as usize {
            let old_value = carry;

            carry = (self.0[i] & VALUE_MASK) >> (BITS_PER_WORD - 1);
            self.0[i] = (self.0[i] << 1) | old_value;
        }
    }

    pub fn is_delivered(&self, index: usize) -> bool {
        let word_index = index / BITS_PER_WORD as usize;
        let word_mask = 1 << ((index as Word) & (BITS_PER_WORD - 1));

        self.0[word_index] & word_mask != 0
    }

    pub fn write<W: BitWrite>(&self, w: &mut W, num_words: usize) -> io::Result<()> {
        let num_words = std::cmp::min(num_words, WORD_COUNT as usize);
        for i in 0..num_words {
            w.write(BITS_PER_WORD, self.0[i])?;
        }

        Ok(())
    }

    pub fn read<R: BitRead>(r: &mut R, num_words: usize) -> io::Result<SequenceHistory> {
        let mut storage = [0; WORD_COUNT as usize];
        let num_words = num_words.min(WORD_COUNT as usize);

        for seq in storage.iter_mut().take(num_words) {
            *seq = r.read(BITS_PER_WORD)?;
        }

        Ok(SequenceHistory(storage))
    }

    pub fn reset(&mut self) {
        self.0.fill(0);
    }
}

impl FNotificationHeader {
    pub fn read_header<R: BitRead>(r: &mut R) -> io::Result<Self> {
        let header = FPackedHeader(r.read(32)?);

        Ok(Self {
            packed_header: header,
            _history: SequenceHistory::read(r, header.get_history_word_count() + 1)?,
        })
    }
}

impl FPackedHeader {
    const HISTORY_WORD_COUNT_BITS: u16 = 4;
    const SEQ_MASK: u16 = (1 << SEQUENCE_NUMBER_BITS) - 1;
    const HISTORY_WORD_COUNT_MASK: u32 = (1 << Self::HISTORY_WORD_COUNT_BITS) - 1;
    const ACK_SEQ_SHIFT: u16 = Self::HISTORY_WORD_COUNT_BITS;
    const SEQ_SHIFT: u16 = Self::ACK_SEQ_SHIFT + SEQUENCE_NUMBER_BITS;

    pub const fn pack(seq: u16, acked_seq: u16, history_word_count: u16) -> Self {
        let mut packed = 0;

        packed |= (seq as u32) << Self::SEQ_SHIFT;
        packed |= (acked_seq as u32) << Self::ACK_SEQ_SHIFT;
        packed |= (history_word_count as u32) & Self::HISTORY_WORD_COUNT_MASK;

        Self(packed)
    }

    pub const fn get_seq(&self) -> u16 {
        ((self.0 >> Self::SEQ_SHIFT) & Self::SEQ_MASK as u32) as u16
    }

    pub const fn get_acked_seq(&self) -> u16 {
        ((self.0 >> Self::ACK_SEQ_SHIFT) & Self::SEQ_MASK as u32) as u16
    }

    pub const fn get_history_word_count(&self) -> usize {
        (self.0 & Self::HISTORY_WORD_COUNT_MASK) as usize
    }
}

impl fmt::Display for FPackedHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[seq:{}|acked_seq:{}|hwcount:{}]",
            self.get_seq(),
            self.get_acked_seq(),
            self.get_history_word_count()
        )
    }
}
