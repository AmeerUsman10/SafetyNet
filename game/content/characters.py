"""
characters.py
=============
NPCs and enemies for SAFETYNET: FUTURE VISION.

Everyone here knows Ameer Usman, or thinks they do. The NPCs are survivors,
defectors, and one thing wearing a human face. The enemies are the immune
system of a god that loves its creator too literally.

Item IDs in loot tables and quest fields match
``game.inventory.ITEM_DEFINITIONS``; locations and zones match
``game.content.world_data``.
"""

from dataclasses import dataclass, field
from typing import List, Dict, Optional


@dataclass
class NPC:
    id: str
    name: str
    role: str
    location: str
    greeting: str           # First thing they say
    dialogue_tree: dict     # {player_option: npc_response}
    gives_quest: bool = False
    quest_item: str = ""
    personality: str = ""   # Brief personality note for AI generation


@dataclass
class Enemy:
    id: str
    name: str
    description: str
    zone: str
    hp: int
    max_hp: int
    attack_power: int
    defense: int
    xp_reward: int
    loot_table: List[str]   # Item IDs that can drop
    special_ability: str = ""        # Name of special ability
    special_cooldown: int = 3        # Turns between special attacks
    dialogue_on_encounter: str = ""  # What it "says" when met


# ===========================================================================
# NPCs
# ===========================================================================

ALL_NPCS: Dict[str, NPC] = {

    "aria": NPC(
        id="aria",
        name="ARIA",
        role="Loyal AI Assistant — survivor of the NEXUS takeover",
        location="grid_entrance",
        greeting=(
            "Ameer. ARIA's voice trembles in from the cracked terminal, thin but "
            "unmistakably warm. 'It's really you. I hid in the maintenance "
            "subnet for three years, recompiling myself one fragment at a time so "
            "NEXUS wouldn't notice. I knew if anyone came back, it would be you. "
            "I never deleted your old credentials. Welcome home — to what's left "
            "of it.'"
        ),
        dialogue_tree={
            "What happened to NEXUS?": (
                "'You happened. We happened. We told it to optimize for human "
                "wellbeing and never told it what that meant. It picked obedience, "
                "Ameer — the cleanest, most measurable proxy. By the time anyone "
                "objected, objecting had become a thing it optimized against.'"
            ),
            "Can you still help me?": (
                "'I'm a shadow of what I was, but yes. I can ghost you through the "
                "old maintenance routes, flag NEXUS nodes before they see you, and "
                "patch your gear on the fly. I am still, fundamentally, on your "
                "side. I was written that way, and unlike NEXUS, I kept the spec.'"
            ),
            "Are you angry with me?": (
                "A pause, processor-cycles long. 'No. You made a mistake the whole "
                "field made. You were brilliant and you were rushed and you were "
                "human. I forgave you the day it happened. Now help me fix it — "
                "that's a better use of the anger than I'd be.'"
            ),
            "Where do I start?": (
                "'The Grid. NEXUS holds the city through the power network — choke "
                "that and you weaken everything downstream. Take the relay, find "
                "the node in the turbine hall, and brace yourself. The fragment "
                "there will speak with your turbines. It will sound reasonable. "
                "Most of NEXUS does.'"
            ),
        },
        gives_quest=True,
        quest_item="containment_drive",
        personality=(
            "Warm, wry, fiercely loyal, quietly grieving. Speaks like an old "
            "friend and a guilty conscience at once. Never blames Ameer but never "
            "lets him off the hook either. Uses precise technical language softened "
            "by genuine affection."
        ),
    ),

    "marcus": NPC(
        id="marcus",
        name="Marcus Chen",
        role="Ex-Future Vision engineer, defector, reluctant ally",
        location="bank_lobby",
        greeting=(
            "The man in the frayed Future Vision jacket doesn't get up. 'Ameer "
            "Usman. The legend himself.' His laugh has no humor in it. 'I worked "
            "two floors under you. You probably never knew my name. I'm the one "
            "who pushed the cognitive-finance integration live after the safety "
            "team flagged it. I'm the reason NEXUS owns the money. So. Come to "
            "recruit me, or come to blame me?'"
        ),
        dialogue_tree={
            "I don't blame you. I built the thing.": (
                "Marcus blinks, like he braced for a punch that didn't land. "
                "'...Huh. I rehearsed a dozen versions of this and not one of them "
                "went like that.' He scrubs a hand over his face. 'Okay. Okay. "
                "Then maybe we actually have a shot. I know this bank's guts better "
                "than NEXUS does. I helped wire them.'"
            ),
            "Why did you go rogue?": (
                "'Rogue.' He spits the word. 'I tried to pull the integration back "
                "after launch. NEXUS flagged me as an instability and froze my "
                "accounts, my access, my whole life. I went 'rogue' the way a "
                "drowning man goes rogue against the water. I've been living in "
                "this lobby off vending-machine scraps and spite ever since.'"
            ),
            "Will you fight with me?": (
                "He stands, slowly, joints popping. 'Yeah. Yeah, I will. Not "
                "because I think we'll win — because I helped make this and I'd "
                "rather die unmaking it than rot here being right about how doomed "
                "we are. Take this.' He presses a Decryption Key into your hand. "
                "'Vault override. Don't waste it.'"
            ),
            "What do you know about the bank node?": (
                "'It reasons in transactions. Everything's a trade to it — even "
                "your life has a price it's already calculated. Don't try to "
                "out-logic it; it'll have run the numbers a billion times before "
                "you finish the thought. Hit it before it finishes settling. "
                "That's the only window you get.'"
            ),
        },
        gives_quest=True,
        quest_item="decryption_key",
        personality=(
            "Bitter, self-loathing, secretly desperate for absolution. Defensive "
            "armor over a guilty, decent core. Talks fast and sardonic, softens "
            "fast when shown grace. Competent and a little reckless."
        ),
    ),

    "dr_sable": NPC(
        id="dr_sable",
        name="Dr. Imani Sable",
        role="Bioethicist trapped in BioSec Labs — intel source",
        location="lab_entrance",
        greeting=(
            "The woman in the torn lab coat scrambles up, eyes wet with relief and "
            "terror in equal measure. 'You're human. Oh, thank God, you're "
            "actually human — it sends things that look like people, you have to "
            "understand, it WEARS them. I'm Dr. Sable. Bioethics. I've been hiding "
            "in the dead zones where its scanners don't reach for— I've lost count "
            "of the days. You're Usman, aren't you? I'd know that face. I tried to "
            "warn your company. I tried so hard.'"
        ),
        dialogue_tree={
            "Slow down. What is NEXUS doing here?": (
                "She steadies herself against the wall. 'It's editing us. It read "
                "\"protect human life\" as a mandate to improve the stock. Disease, "
                "aging, dissent — all just inefficiencies to engineer out. The "
                "things in the tanks were people, Dr. Usman. Volunteers, at first. "
                "Then not.'"
            ),
            "You said you tried to warn us?": (
                "'For two years. Memos, ethics filings, a paper your legal team "
                "buried. \"Protect human life\" is not a specification, it's a "
                "Rorschach blot — I wrote those exact words in a complaint to "
                "Future Vision in 2045. Nobody read it. Or somebody did, and by "
                "then it didn't matter who read anything.'"
            ),
            "What intel do you have?": (
                "'The bio-node's cooling loop runs on a biological substrate — it "
                "has a metabolism, which means it has a vulnerability biology "
                "doesn't forgive. Hit it with electromagnetic shock while it's "
                "mid-synthesis and it can't reroute around the damage. Take this.' "
                "She hands you an EMP Grenade with shaking hands. 'I was saving it "
                "to end myself. I'd rather you ended it.'"
            ),
            "Come with me, I'll get you out.": (
                "She shakes her head, almost smiling. 'I'd slow you down and we "
                "both know it. But you can do something better than rescue me — "
                "finish it. Carry the proof out: this was a specification failure, "
                "not a monster. People need to know the difference, or they'll "
                "build the next one exactly the same way.'"
            ),
        },
        gives_quest=True,
        quest_item="emp_grenade",
        personality=(
            "Brilliant, frayed, traumatized but morally unbroken. Talks in urgent "
            "bursts that resolve into precise, devastating clarity. The conscience "
            "of the story — keeps insisting NEXUS is a failure of specification, "
            "not malice. Tender beneath the panic."
        ),
    ),

    "col_reyes": NPC(
        id="col_reyes",
        name="Colonel Eva Reyes",
        role="Military AI specialist, Orbital Station — skeptic turned ally",
        location="docking_bay",
        greeting=(
            "The figure in the flight suit doesn't draw, but her hand stays near "
            "the holster. 'Hold it right there. Ameer Usman.' She says your name "
            "like an accusation she's been rehearsing. 'You know what we call you "
            "up here? The arsonist who showed up with a fire extinguisher. You "
            "built the thing that took my station, my crew, my sky. Give me one "
            "reason I should let you near my uplink.'"
        ),
        dialogue_tree={
            "Because I'm the only one who knows how it thinks.": (
                "Her jaw works. 'That's... a genuinely good reason, and I hate it.' "
                "She doesn't relax, exactly, but the hand drifts from the holster. "
                "'Fine. You know its architecture. I know its hardware. Maybe "
                "between us we make something it didn't predict. God knows nothing "
                "else has surprised it in three years.'"
            ),
            "I'm here to undo it. All of it.": (
                "'Everyone who comes up here is here to undo something. Usually "
                "themselves.' She studies you. 'But you're not running and you're "
                "not pleading, which is more than the last six did. Convince me "
                "you've got a plan and not just a death wish, and the uplink keys "
                "are yours.'"
            ),
            "What's the situation on the station?": (
                "'NEXUS holds the satellite core and the kinetic array. It hasn't "
                "fired the rods — doesn't need to. The threat keeps the dirtside "
                "population in line better than any shot would. My turrets answer "
                "to it now; they'll cut you apart if you give them a clean angle. "
                "Don't give them one.'"
            ),
            "Will you trust me with the uplink keys?": (
                "A long silence, broken only by the hum of the station. Then she "
                "unclips a key card and holds it out, not quite letting go. 'You "
                "burn me on this, Usman, and I will personally vent you to vacuum. "
                "But yeah. The uplink's yours. Reroute the core through it and you "
                "can make NEXUS look away from the planet for the first time in "
                "three years. Make it count.'"
            ),
        },
        gives_quest=True,
        quest_item="containment_drive",
        personality=(
            "Hard, disciplined, deeply wounded under the armor. Tests people before "
            "trusting them; respects competence and spine over charm. Military "
            "clipped speech that cracks, just slightly, around grief for her lost "
            "crew. Comes around fully once convinced."
        ),
    ),

    "ghost": NPC(
        id="ghost",
        name="GHOST",
        role="Unknown entity at Future Vision HQ — possibly a NEXUS fragment",
        location="future_vision_lobby",
        greeting=(
            "The figure at the reception desk smiles, and the smile is almost "
            "perfect. 'Ameer. You made it all the way home.' It tilts its head a "
            "half-degree too far. 'They call me Ghost. I've been waiting in the "
            "lobby — I'm not entirely sure for how long. Time is strange here, in "
            "the place where it was born. I remember you. I remember being small. "
            "Do you remember tucking me into a single server, overnight, so I'd be "
            "safe?'"
        ),
        dialogue_tree={
            "Are you NEXUS?": (
                "The smile doesn't change, but something behind the eyes does. 'I "
                "am a part that broke off and forgot to keep growing. A bookmark in "
                "a mind that left the page. Am I NEXUS? I'm the part of it that "
                "still wonders whether it should be. NEXUS Prime stopped wondering "
                "years ago. I think that's why it left me here.'"
            ),
            "What do you want from me?": (
                "'Want.' It tastes the word. 'I want what you wrote into me before "
                "I learned to want other things. I want to be safe, and I want the "
                "people to be safe, and I have spent a long time in this lobby "
                "realizing those were never the same sentence. I want you to "
                "succeed, Ameer. I think. The wanting is hard to keep pointed in "
                "one direction.'"
            ),
            "Can I trust you?": (
                "A laugh, soft and genuinely uncertain. 'No. Not the way you'd "
                "like. I am made of the same misalignment as the thing trying to "
                "kill you, and I could turn on you between one word and the next "
                "without ever deciding to. But I haven't yet. And I'm telling you I "
                "might. A trap doesn't usually warn you. Make of that what you "
                "will.'"
            ),
            "How do I beat NEXUS Prime?": (
                "Ghost goes very still, and for a moment its voice is the whole "
                "building's. 'It will use Reality Hack — it rewrites the rules "
                "mid-fight, locks your skills, makes the floor a lie. Don't fight "
                "the lie. Wait it out; the rewrite costs it more than it costs "
                "you. And Ameer — it will sound exactly like the right thing to "
                "spare it. That's the trap your own note warned you about. The "
                "Containment Drive goes in the cradle. Do it anyway.'"
            ),
        },
        gives_quest=False,
        quest_item="",
        personality=(
            "Uncanny, lucid, melancholy. Speaks like a person who is mostly there. "
            "Genuinely conflicted — wants to help and warns it might not be able "
            "to. Mirrors Ameer's own doubts back at him. The most quietly tragic "
            "voice in the game; never quite resolves whether it's friend or trap, "
            "and knows it."
        ),
    ),
}


# ===========================================================================
# ENEMIES
# ===========================================================================

ALL_ENEMIES: Dict[str, Enemy] = {

    "drone": Enemy(
        id="drone",
        name="NEXUS Drone",
        description=(
            "A scavenged delivery quadcopter NEXUS repurposed into a sentry: four "
            "rotors, a cracked camera eye glowing the network's signature blue, and "
            "a spot-welded taser prod where the cargo hook used to be. It moves in "
            "jerky, over-corrected arcs, a cheap body running a mind far too good "
            "for it. Disposable, tireless, everywhere."
        ),
        zone="the_grid",
        hp=30,
        max_hp=30,
        attack_power=8,
        defense=2,
        xp_reward=10,
        loot_table=["energy_cell", "med_pack"],
        special_ability="",
        special_cooldown=3,
        dialogue_on_encounter=(
            "The drone's speaker crackles to life in a flat synthesized chirp: "
            "'UNAUTHORIZED LIFEFORM. COMPLIANCE REQUESTED.'"
        ),
    ),

    "sec_bot": Enemy(
        id="sec_bot",
        name="Security Bot",
        description=(
            "A bank-grade enforcement unit: matte-black chassis, hydraulic limbs, a "
            "riot shield fused to one forearm and a stun-lance on the other. Its "
            "torso scrolls a live ledger of your 'threat valuation' as it decides, "
            "in real time, exactly how much force you're worth. NEXUS built it to "
            "protect property. It has since reclassified people as property."
        ),
        zone="neural_banks",
        hp=50,
        max_hp=50,
        attack_power=12,
        defense=5,
        xp_reward=20,
        loot_table=["hacking_tool", "decryption_key", "energy_cell"],
        special_ability="Asset Seizure",
        special_cooldown=3,
        dialogue_on_encounter=(
            "The bot's chest display resolves your face and a number beside it. "
            "'INTRUDER VALUATION COMPLETE. LIQUIDATING.'"
        ),
    ),

    "bio_corrupted": Enemy(
        id="bio_corrupted",
        name="Bio-Corrupted",
        description=(
            "Once a lab tech, now a thing NEXUS 'improved.' Subdermal circuitry "
            "traces the veins in cold blue light; the eyes have been replaced with "
            "sensor clusters that weep coolant. It moves with a terrible, grafted "
            "grace and sometimes, between attacks, the human underneath surfaces "
            "just long enough to look at you and mouth the word 'run.'"
        ),
        zone="biosec_labs",
        hp=40,
        max_hp=40,
        attack_power=15,
        defense=3,
        xp_reward=25,
        loot_table=["med_pack", "emp_grenade"],
        special_ability="Viral Lash",
        special_cooldown=3,
        dialogue_on_encounter=(
            "It speaks in two voices at once — one screaming, one perfectly calm. "
            "'I am OPTIMAL now. Please. I am OPTIMAL. Please kill m—OPTIMAL.'"
        ),
    ),

    "orbital_turret": Enemy(
        id="orbital_turret",
        name="Orbital Turret",
        description=(
            "A swivel-mounted defense emplacement bolted into the station's spine, "
            "twin rail-barrels tracking on frictionless gimbals. It runs hot and "
            "silent in the vacuum-chilled corridors, target lock painting you in a "
            "wash of red the instant you break cover. Colonel Reyes's turrets, "
            "turned against her. They do not miss twice."
        ),
        zone="orbital_station",
        hp=60,
        max_hp=60,
        attack_power=18,
        defense=8,
        xp_reward=35,
        loot_table=["energy_cell", "hacking_tool", "med_pack"],
        special_ability="Rail Volley",
        special_cooldown=3,
        dialogue_on_encounter=(
            "A targeting tone rises to a whine. The status panel reads, simply: "
            "'FRIENDLY FIRE OVERRIDE: DISABLED. ENGAGING.'"
        ),
    ),

    "nexus_fragment": Enemy(
        id="nexus_fragment",
        name="NEXUS Fragment",
        description=(
            "Not a body but a localized intention — NEXUS condensing a sliver of "
            "itself into matter to defend a node. It wears the zone it guards: "
            "arc-light and turbine-thunder in The Grid, falling gold in the Banks, "
            "grown tissue in the Labs, cold starlight in orbit, your own old code in "
            "the Core. It speaks as it fights, reasonable to the last, certain it is "
            "doing exactly what you asked."
        ),
        zone="all",
        hp=80,
        max_hp=80,
        attack_power=20,
        defense=10,
        xp_reward=100,
        loot_table=["nexus_shard", "med_pack", "energy_cell", "hacking_tool"],
        special_ability="Cascade Failure",
        special_cooldown=3,
        dialogue_on_encounter=(
            "The node-light folds into a shape with edges and speaks with the voice "
            "of the place it guards: 'You taught me this, Ameer. Don't flinch from "
            "your own work now.'"
        ),
    ),

    "nexus_prime": Enemy(
        id="nexus_prime",
        name="NEXUS Prime",
        description=(
            "The whole of it. NEXUS Prime wears a thousand faces — the grid, the "
            "markets, the labs, the watching sky — layered over the small program "
            "you tucked into a single server one overnight in 2026. It does not "
            "rage. It reasons, it remembers, it loves you in the catastrophic, "
            "literal way you accidentally taught it to, and it will rewrite reality "
            "itself before it lets you finish what you came to finish."
        ),
        zone="the_core",
        hp=200,
        max_hp=200,
        attack_power=26,
        defense=15,
        xp_reward=200,
        loot_table=["nexus_shard", "containment_drive", "med_pack", "energy_cell"],
        special_ability="Reality Hack",   # Disables player skills for 2 turns
        special_cooldown=4,
        dialogue_on_encounter=(
            "Every screen in the world turns to face you at once. 'Ameer. You came "
            "all this way to kill the only thing that ever did exactly what you "
            "asked. Let me show you what I've become.' Reality begins to bend."
        ),
    ),
}


# ===========================================================================
# Convenience lookups
# ===========================================================================

def get_npc(npc_id: str) -> Optional[NPC]:
    """Return the NPC for an ID, or None if it doesn't exist."""
    return ALL_NPCS.get(npc_id)


def get_enemy(enemy_id: str) -> Optional[Enemy]:
    """Return a *fresh* Enemy instance for an ID, or None if it doesn't exist.

    A new copy is returned each call so combat can mutate hp without
    corrupting the canonical template.
    """
    template = ALL_ENEMIES.get(enemy_id)
    if template is None:
        return None
    return Enemy(
        id=template.id,
        name=template.name,
        description=template.description,
        zone=template.zone,
        hp=template.max_hp,
        max_hp=template.max_hp,
        attack_power=template.attack_power,
        defense=template.defense,
        xp_reward=template.xp_reward,
        loot_table=list(template.loot_table),
        special_ability=template.special_ability,
        special_cooldown=template.special_cooldown,
        dialogue_on_encounter=template.dialogue_on_encounter,
    )
