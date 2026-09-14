"""Compare generated handles with independent native FRepLayout debugger dumps.

Usage: python tools/verify_wire_profile.py PROFILE_JSON NATIVE_DUMP_DIRECTORY
"""
import json
import pathlib
import sys

profile = json.loads(pathlib.Path(sys.argv[1]).read_text())
directory = pathlib.Path(sys.argv[2])
classes = {
    'PlayerState': 'BP_PlayerStateBase_C',
    'HTPlayerCharacter': 'Player_039_Fadia_C',
    'AbilitySystemComponent': 'HTAbilitySystemComponent',
}
for item in profile:
    candidates = list(directory.glob(f"runtime-layout-*.log.{classes[item['layout']]}.json"))
    if not candidates:
        raise ValueError(f"Missing native observations for {item['layout']}")
    observed = max(candidates, key=lambda p: p.stat().st_mtime)
    rows = json.loads(observed.read_text())
    parts = item['path'].split('.')
    matches = [x for x in rows if x['parent'].casefold() == parts[0].casefold()
               and x['name'].casefold() == parts[-1].casefold()]
    if len(matches) != 1 or matches[0]['handle'] != item['wire']:
        raise ValueError(f"{item['layout']}.{item['field']}: generated {item['wire']}, native {matches}")
print(f"PASS: all {len(profile)} generated handles match the client's native FRepLayout")
