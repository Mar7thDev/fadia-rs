use fadia_codegen::HottaReplicatedObject;
use fadia_engine::{
    util::{FStringWriteExt, WritePrimitivesExt},
    vector::FVector3d,
};

use super::HottaReplicatedProperty;

// Typos inside of #[property] are their typos

#[derive(Debug, HottaReplicatedObject)]
pub struct HottaPlayerState {
    #[property("HasNamed")]
    pub has_named: bool,
    #[property("bCanMarkIcon")]
    pub can_mark_icon: bool,
    #[property("CurDivinationDataArray")]
    pub cur_divination_data_array: Vec<()>,
    #[property("UnlockAvartaIds")]
    pub unlock_avatar_ids: Vec<String>,
    #[property("UnlockMedalIds")]
    pub unlock_medal_ids: Vec<String>,
    #[property("MedalIds")]
    pub medal_ids: Vec<String>,
    #[property("UnlockAvartaFrameIds")]
    pub unlock_avatar_frame_ids: Vec<String>,
    #[property("PlayerInfoSelections")]
    pub player_info_selections: Vec<String>,
    #[property("FinishedGuide")]
    pub finished_guide: Vec<String>,
    #[property("TrigGuide")]
    pub trig_guide: Vec<String>,
    #[property("LikeabilityInfos")]
    pub likeability_infos: Vec<()>,
    #[property("Gashapons")]
    pub gashapons: Vec<()>,
    #[property("ChatItemBufferData")]
    pub chat_item_buffer_data: Vec<()>,
    #[property("LikeabilityChatGroupSaveDataArray")]
    pub likeability_chat_group_save_data_array: Vec<()>,
    #[property("ReceiveCloneSystemContainIDs")]
    pub receive_clone_system_contain_ids: Vec<()>,
    #[property("WorldLevel")]
    pub world_level: u32,
    #[property("MaxWorldLevel")]
    pub max_world_level: u32,
    #[property("LastAjustWorldLevelTime")]
    pub last_adjust_world_level_time: u64,
    #[property("ItemEffectCDData")]
    pub item_effect_cd_data: Vec<()>,
    #[property("OwnedMusicList")]
    pub owned_music_list: Vec<String>,
    #[property("CurrentDisplayRealEstate")]
    pub current_display_real_estate: String,
    #[property("FunctionUnlockArray")]
    pub function_unlock_array: Vec<String>,
    #[property("NPCSpecialGiftList")]
    pub npc_special_gift_list: Vec<()>,
    #[property("CurrentBakStamina")]
    pub current_bak_stamina: u32,
    #[property("LastRecoveryBakStaminaTime")]
    pub last_recovery_bak_stamina_time: u64,
    #[property("BakStaminaAutoRecoveryValue")]
    pub bak_stamina_auto_recovery_value: u32,
    #[property("BakStaminaAutoRecoveryInterval")]
    pub bak_stamina_auto_recovery_interval: u32,
    #[property("WeeklyUnlockCloneIDs")]
    pub weekly_unlock_clone_ids: Vec<()>,
    #[property("WeathervaneID")]
    pub weathervane_id: String,
    #[property("CurDecorationPlans")]
    pub cur_decoration_plans: Vec<RoomDecorationPlans>,
    #[property("SaveDecorationPlan")]
    pub save_decoration_plan: Vec<()>,
    #[property("FurnitureUsedItem")]
    pub furniture_used_item: Vec<()>,
    #[property("SaveFurnitureTransform")]
    pub save_furniture_transform: Vec<()>,
    #[property("RoomLoad")]
    pub room_load: Vec<()>,
    #[property("TakeOrdersSaveData")]
    pub take_orders_save_data: Vec<TakeOrdersSaveData>,
    #[property("CommonStaminaSaveDatas")]
    pub common_stamina_save_datas: Vec<()>,
    #[property("OnlineChallengeData")]
    pub online_challenge_data: u64,
    #[property("Dolls")]
    pub dolls: Vec<()>,
    #[property("Racing1V1ModeUnlocked")]
    pub racing_1v1_mode_unlocked: bool,
    #[property("RacingPVESuccessed")]
    pub racing_pve_successed: bool,
    #[property("UnlockAppearanceIds")]
    pub unlock_appearance_ids: Vec<()>,
    #[property("FurnitureActivateStates")]
    pub furniture_activate_states: Vec<()>,
    #[property("PlacementVehcialData")]
    pub placement_vehicle_data: Vec<()>,
    #[property("m_BeforeSinBossTransform")]
    pub before_sin_boss_transform: Transform,
    #[property("SocialSetting")]
    pub social_setting: SocialSetting,
    #[property("AllAuctionDataInfos")]
    pub all_auction_data_infos: Vec<()>,
    #[property("AllAuctionBuyRecords")]
    pub all_auction_buy_records: Vec<()>,
    #[property("AuctionBeginTimeStamp")]
    pub auction_begin_time_stamp: u64,
    #[property("AuctionEndTimeStamp")]
    pub auction_end_time_stamp: u64,
    #[property("AuctionCloseTimeStamp")]
    pub auction_close_time_stamp: u64,
    #[property("CurAuctionID")]
    pub cur_auction_id: String,
    #[property("NextNpcID")]
    pub next_npc_id: String,
    #[property("AuctionOpenTimeStamp")]
    pub auction_open_time_stamp: u64,
    #[property("NpcBiddingCount")]
    pub npc_bidding_count: u32,
    #[property("StaticAuctionShopRefreshCount")]
    pub static_auction_shop_refresh_count: u32,
    #[property("AuctionShopRefreshRecord")]
    pub auction_shop_refresh_record: u32,
    #[property("StaticExtraAuctionItemCount")]
    pub static_extra_auction_item_count: u32,
    #[property("NextNpcBiddingTimeStamp")]
    pub next_npc_bidding_time_stamp: u64,
    #[property("ComplementItems")]
    pub complement_items: Vec<()>,
    #[property("GoldCloneSelectedFriendRoleID")]
    pub gold_clone_selected_friend_role_id: u64,
    #[property("CloneSystemChallengeSuccessIDs")]
    pub clone_system_challenge_success_ids: Vec<()>,
}

#[derive(Debug)]
pub struct RoomDecorationPlans(pub String, pub String);

#[derive(Debug)]
pub struct TakeOrdersSaveData {
    pub point_id: String,
    pub order_id: String,
}

#[derive(Debug)]
pub struct Transform {
    pub position: FVector3d,
    pub rotation: FVector3d,
    pub scale: FVector3d,
}

#[derive(Debug)]
pub struct SocialSetting {
    pub is_show_info: bool,
    pub can_be_friend: bool,
    pub is_show_birthday: bool,
    pub is_show_estate: bool,
    pub is_show_vehicle: bool,
    pub is_show_player_list: bool,
}

impl HottaReplicatedProperty for RoomDecorationPlans {
    fn replicate_impl(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        w.write_string(&self.0)?;
        w.write_string(&self.1)
    }
}

impl HottaReplicatedProperty for TakeOrdersSaveData {
    fn replicate_impl(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        w.write_string(&self.point_id)?;
        w.write_string(&self.order_id)?;
        w.write_u16(0)?; // ??
        w.write_u8(0) // ??
    }
}

impl HottaReplicatedProperty for Transform {
    fn replicate_impl(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        w.write_vector(&self.position)?;
        w.write_vector(&self.rotation)?;
        w.write_vector(&self.scale)
    }
}

impl HottaReplicatedProperty for SocialSetting {
    fn replicate_impl(&self, w: &mut fadia_engine::util::OutBitWriter) -> std::io::Result<()> {
        self.is_show_info.replicate_impl(w)?;
        self.can_be_friend.replicate_impl(w)?;
        self.is_show_birthday.replicate_impl(w)?;
        self.is_show_estate.replicate_impl(w)?;
        self.is_show_vehicle.replicate_impl(w)?;
        self.is_show_player_list.replicate_impl(w)
    }
}

impl Default for HottaPlayerState {
    fn default() -> Self {
        Self {
            has_named: Default::default(),
            can_mark_icon: Default::default(),
            cur_divination_data_array: Default::default(),
            unlock_avatar_ids: vec![String::from("1")],
            unlock_medal_ids: Default::default(),
            medal_ids: Vec::from([
                String::from("None"),
                String::from("None"),
                String::from("None"),
                String::from("None"),
            ]),
            unlock_avatar_frame_ids: Default::default(),
            player_info_selections: Vec::from([
                String::from("None"),
                String::from("None"),
                String::from("None"),
                String::from("None"),
                String::from("None"),
            ]),
            finished_guide: Default::default(),
            trig_guide: Default::default(),
            likeability_infos: Default::default(),
            gashapons: Default::default(),
            chat_item_buffer_data: Default::default(),
            likeability_chat_group_save_data_array: Default::default(),
            receive_clone_system_contain_ids: Default::default(),
            world_level: Default::default(),
            max_world_level: Default::default(),
            last_adjust_world_level_time: Default::default(),
            item_effect_cd_data: Default::default(),
            owned_music_list: Default::default(),
            current_display_real_estate: Default::default(),
            function_unlock_array: Default::default(),
            npc_special_gift_list: Default::default(),
            current_bak_stamina: Default::default(),
            last_recovery_bak_stamina_time: Default::default(),
            bak_stamina_auto_recovery_value: Default::default(),
            bak_stamina_auto_recovery_interval: Default::default(),
            weekly_unlock_clone_ids: Default::default(),
            weathervane_id: String::from("1004"),
            cur_decoration_plans: Default::default(),
            save_decoration_plan: Default::default(),
            furniture_used_item: Default::default(),
            save_furniture_transform: Default::default(),
            room_load: Default::default(),
            take_orders_save_data: Default::default(),
            common_stamina_save_datas: Default::default(),
            online_challenge_data: Default::default(),
            dolls: Default::default(),
            racing_1v1_mode_unlocked: Default::default(),
            racing_pve_successed: Default::default(),
            unlock_appearance_ids: Default::default(),
            furniture_activate_states: Default::default(),
            placement_vehicle_data: Default::default(),
            before_sin_boss_transform: Default::default(),
            social_setting: SocialSetting {
                is_show_info: true,
                can_be_friend: true,
                is_show_birthday: true,
                is_show_estate: true,
                is_show_vehicle: true,
                is_show_player_list: true,
            },
            all_auction_data_infos: Default::default(),
            all_auction_buy_records: Default::default(),
            auction_begin_time_stamp: Default::default(),
            auction_end_time_stamp: Default::default(),
            auction_close_time_stamp: Default::default(),
            cur_auction_id: Default::default(),
            next_npc_id: Default::default(),
            auction_open_time_stamp: Default::default(),
            npc_bidding_count: Default::default(),
            static_auction_shop_refresh_count: Default::default(),
            auction_shop_refresh_record: Default::default(),
            static_extra_auction_item_count: Default::default(),
            next_npc_bidding_time_stamp: Default::default(),
            complement_items: Default::default(),
            gold_clone_selected_friend_role_id: Default::default(),
            clone_system_challenge_success_ids: Default::default(),
        }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: FVector3d::default(),
            rotation: FVector3d::default(),
            scale: FVector3d::new(1.0, 1.0, 1.0),
        }
    }
}
