"""Build explicit checklist coverage and cross-build lead lists, never capabilities.

Usage: python scripts/nhl27/build_evidence.py ROOT27 ROOT24
Name matches are leads. The generated files deliberately retain this distinction.
"""
import collections
import json
import re
import sqlite3
import sys
from pathlib import Path

CHECKLIST = [
 ('01_league_size',r'TeamsInLeague|FeLeagueCreator|CustomLeague'),
 ('02_add_leagues',r'LeagueFactory|LeagueType|LeagueRegistry|pwhlschedule'),
 ('03_short_rosters',r'MinNumberOfPlayable|MaxPlayableRoster|MinPlayableRoster|MaxTotalRoster|roster:is-valid'),
 ('04_full_league_control',r'FeTeamControlType|ModeCreationUserTeams|NumUserTeams|UserTeamList|FranchiseConversation'),
 ('05_historical_rules',r'TwoLinesOffside|TwoLinePass|Rule_JailBreak|NHL_Jailbreak|NoForwardPass'),
 ('06_leagues_in_franchise',r'LeagueFactory|custom-leagues:|League_TeamsInLeague'),
 ('07_strategies',r'StrategyPlaybook|nhl-data:strategy:|strategyplaybook|strategysliders'),
 ('08_ai',r'^g.*(?:Poke|Deke|Forecheck|Checking|DynamicTendency)'),
 ('09_pressure',r'^g.*Pressure|Pressure.*Attribute|Setting_Pressure'),
 ('10_attribute_effects',r'gAttrib|gPlayerAttribute|gGoalieAttribute|Attribute.*Curve'),
 ('11_heads_portraits',r'HeadGeometry|HeadTexture|HeadVariant|PortraitId|HasPortrait|Player_Art'),
 ('12_sliders',r'^Tweak_|^Setting_Sim_|Slider.*Curve'),
 ('13_ones',r'ones_now|ones_h2h|gOnesCPU|EliminatorOnes'),
 ('14_hits',r'gTimeAfterReleaseForHitToCount|gProximityToPuckForHitToCount|InitialCheck'),
 ('15_turnovers',r'gTurnover|GameEvent::Takeaway|GameEvent::Giveaway'),
 ('16_simulation',r'simengine.brk|nhl_gm_simulation|FeSimulation|SimEngine|SimPlayer'),
 ('17_capacities',r'Max.*(?:Players|Teams|Leagues|Jersey|Equipment|Looks)|MaxTotalRosterSize'),
 ('18_allstar',r'DYNASTY_ALLSTARGAME|TodayIsAllstarGame|GAMETYPE_ALLSTAR'),
 ('19_pwhl',r'PWHL|JailBreak|Jailbreak|jailbreak'),
 ('20_be_a_pro_updates',r'BeAProEnergySystem|BeAProState::|BeAProPerks::|BeAProPlayer::.*Objectives|BeAPro.*Endorsement'),
 ('21_animations_dekes',r'Deke|SkillStick|MotionMatching|AnimGraph|AnimationGraph|AnimClip|StickTrick|PuckControl'),
 ('22_presentation',r'PreGame|Pregame|Presentation|Broadcast|ReplayDirector|CameraDirector|GoalCelebration|PlayerIntro|TeamIntro'),
 ('23_play_by_play',r'PlayByPlay|Play.By.Play|Commentary|Commentator|SpeechEvent|SpeechBank|Announcer|VoiceOver'),
]


def run(root, old):
    roots={'nhl27':root,'nhl24_base':old/'base','nhl24_160':old/'update'}
    dbs={k:sqlite3.connect(f'file:{(p/"index.sqlite").as_posix()}?mode=ro',uri=True) for k,p in roots.items()}
    names={k:dict((s,va) for va,s in d.execute('SELECT va,text FROM strings')) for k,d in dbs.items()}
    results=[]
    for key,pat in CHECKLIST:
        rx=re.compile(pat,re.I)
        hits=[]
        for text,va in names['nhl27'].items():
            if not rx.search(text):continue
            item={'name':text,'builds':{}}
            for build,d in dbs.items():
                addr=names[build].get(text)
                if addr is None:continue
                refs=[{'va':hex(src),'preceding_unwind_start':hex(f) if f is not None else None}
                      for src,f in d.execute('SELECT src,(SELECT max(start) FROM functions WHERE start<=src) FROM refs WHERE dst=?',(addr,))]
                item['builds'][build]={'string_va':hex(addr),'candidate_refs':refs}
            hits.append(item)
        results.append({'id':key,'stage':'name/reference leads; see dossier for individually traced findings',
                        'matches':len(hits),'names_absent_from_both_nhl24_indexes':sum(len(x['builds'])==1 for x in hits),'leads':hits})
    (root/'checklist-evidence.json').write_text(json.dumps(results,indent=2))
    prefixes=['nhl-data:strategy:','nhl-data:franchise:conversation:','nhl-data:contracts:player-clause:']
    services={p:{k:sorted(s for s in v if s.startswith(p)) for k,v in names.items()} for p in prefixes}
    (root/'service-comparison.json').write_text(json.dumps(services,indent=2))
    assets={}
    for label,p in [('nhl27',root/'assets'),('nhl24_base',root/'nhl24-base-assets'),('nhl24_160',root/'nhl24-update-assets')]:
        rows=[json.loads(l) for l in (p/'bundles.jsonl').open()]
        groups={}
        for key,pat in [('strategy',r'strateg|playbook'),('franchise_ui',r'/ui/.*(?:franchise|begm|dynasty|contract|negotiat)'),
                        ('bap_ui',r'/ui/.*(?:beapro|be_a_pro|bap|career)'),('pwhl',r'pwhl|jailbreak'),
                        ('animations_dekes',r'/anims?/|animation|deke|skillstick|motionmatch'),
                        ('presentation',r'pregame|presentation|broadcast|replay|celebration|playerintro|teamintro'),
                        ('play_by_play',r'playbyplay|play.by.play|commentary|commentator|speech|announcer|voiceover')]:
            groups[key]=sorted(set(x['name'] for x in rows if re.search(pat,x['name'],re.I)))
        base={x['name'] for x in rows if x['layer']=='Data'};patch={x['name'] for x in rows if x['layer']=='Patch'}
        assets[label]={'occurrences':len(rows),'unique_names':len(base|patch),'base_patch_name_overlap':len(base&patch),
                       'groups':groups,'group_counts':{k:len(v) for k,v in groups.items()}}
    (root/'asset-comparison.json').write_text(json.dumps(assets,indent=2))
    print(json.dumps({'checklist':[{k:v for k,v in r.items() if k!='leads'} for r in results],
                      'assets':{k:{a:b for a,b in v.items() if a!='groups'} for k,v in assets.items()}},indent=2))


if __name__=='__main__':run(Path(sys.argv[1]),Path(sys.argv[2]))
