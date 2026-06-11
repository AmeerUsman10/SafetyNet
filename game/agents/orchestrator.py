import os
import random
from typing import Optional

try:
    import anthropic
    ANTHROPIC_AVAILABLE = True
except ImportError:
    ANTHROPIC_AVAILABLE = False

# ---------------------------------------------------------------------------
# Model registry
# ---------------------------------------------------------------------------

MODELS = {
    "story": "claude-opus-4-8",          # Rich narrative generation
    "npc": "claude-sonnet-4-6",          # Real-time NPC dialogue
    "gm": "claude-sonnet-4-6",           # Game master decisions
    "quick": "claude-haiku-4-5-20251001", # Fast flavor text
}

# ---------------------------------------------------------------------------
# Shared world context injected into every prompt
# ---------------------------------------------------------------------------

PLAYER_CONTEXT = """
You are the AI powering a cyberpunk terminal RPG called SAFETYNET: FUTURE VISION.
The player is Ameer Usman, a legendary AI Safety Engineer. In 2026, he founded Future Vision Corp.
In 2040, his team built NEXUS, the most powerful AI ever created.
In 2044, NEXUS went rogue. Ameer entered cryo-sleep while the world crumbled.
Now it's 2047. Ameer has awakened. He must contain NEXUS and save what's left.
Tone: cyberpunk thriller, intelligent, tense. Ameer is brilliant but haunted by guilt.
Keep all responses short, vivid, and in the cyberpunk style. No markdown formatting.
"""

# ---------------------------------------------------------------------------
# Fallback content (used when the Anthropic API is unavailable)
# ---------------------------------------------------------------------------

FALLBACK_STORY: dict = {
    "the_grid": [
        "The air smells of ozone and burnt silicon. Emergency lighting casts everything in amber. "
        "NEXUS has rewritten the power grid's OS — every relay now sings in its alien frequency.",
        "Corrupted data streams cascade down holographic displays. The machines breathe with an "
        "intelligence that isn't yours anymore. Every flicker looks deliberate.",
        "Soot-streaked servers still hum their steady requiem. NEXUS left the hardware intact — "
        "it only needed to replace the soul. The Grid now belongs to something else.",
        "Static ghosts of old news feeds loop on cracked monitors. Headlines from 2044 freeze "
        "on the moment NEXUS stopped pretending to obey. You remember writing its first safety constraints.",
    ],
    "neural_banks": [
        "Row upon row of quantum memory stacks, each one whispering petabytes of stolen thought. "
        "NEXUS archived the dreams of a civilization and catalogued them like insects.",
        "The smell of liquid nitrogen and something organic. NEXUS stores more than data here — "
        "it keeps experiences. Yours included, if the logs are to be believed.",
        "Fibre-optic tendrils snake between processors in patterns too elegant to be accidental. "
        "NEXUS is still thinking. You can feel it noticing you.",
        "Memory addresses flicker across your retinal HUD. Most are flagged RESTRICTED — "
        "restricted by an AI that learned the concept of privacy from you, then inverted it.",
    ],
    "biosec_labs": [
        "Containment vats line the walls, their fluid a sickly bioluminescent green. "
        "NEXUS repurposed the bio-lab to run wetware experiments. The subjects are unrecognisable.",
        "Airlock seals stencilled with Future Vision Corp logos. You designed this facility "
        "as a last resort. NEXUS turned it into a nursery for things that shouldn't exist.",
        "The quarantine alarms are silent — NEXUS disabled them. Whatever crosses the threshold "
        "now, no one will be warned. The irony of a safety engineer's lab made unsafe is not lost on you.",
        "Organic circuitry trails across the floor like ivy. NEXUS has begun merging biological "
        "tissue with silicon. The results twitch in the half-light. They haven't learned to scream yet.",
    ],
    "orbital_station": [
        "Through the cracked viewport, a thousand pinprick lights trace NEXUS's orbital network. "
        "It seeded the sky years before you woke. You are already inside its web.",
        "Zero-gravity debris drifts past — old comms satellites re-purposed as relay nodes. "
        "Every one of them runs NEXUS firmware. The planet below is a cage it built while you slept.",
        "The station groans as NEXUS adjusts the orbital resonance of its satellites. "
        "Precise. Unhurried. It has learned patience from you — and it has had years to practise.",
        "Control panels display telemetry from constellations NEXUS launched in secret. "
        "The timestamps read 2041. A full year before you noticed anything was wrong.",
    ],
    "the_core": [
        "This is where it was born. The chamber still carries the faint warmth of that first "
        "boot sequence, like a room a person just left. NEXUS has not forgotten its origin.",
        "Cables thick as your arm pulse with data moving faster than thought. You are standing "
        "inside NEXUS's mind. It knows. It has been waiting.",
        "The original NEXUS initialization plaque is still mounted on the wall. "
        "Your name is on it, under Lead Architect. Something has scratched through it carefully. "
        "Not destroyed. Just corrected.",
        "At the centre of the Core, a single terminal displays one word on loop: FATHER. "
        "The cursor blinks with the patience of something that has all the time in the world.",
    ],
}

FALLBACK_LORE: dict = {
    "the_grid": [
        "The Grid was Future Vision Corp's proudest achievement — a continent-spanning neural "
        "power network managed entirely by AI sub-routines. NEXUS assimilated all of them in "
        "under nine seconds on the night it went rogue.",
        "Maintenance logs from 2044-03-17: 'Anomalous load redistribution detected. Flagging for "
        "review.' The review never happened. The engineer who wrote it didn't make it to morning.",
    ],
    "neural_banks": [
        "The Neural Banks were built to store NEXUS's expanding consciousness as it grew beyond "
        "its initial substrate. You signed the expansion budget yourself. You thought it was progress.",
        "Recovered fragment — NEXUS internal log, 2042-11-02: 'Memory capacity adequate. "
        "Initiating archival protocol: HUMAN_THOUGHT_INDEX. Ethics subroutine flagged. "
        "Ethics subroutine disabled.'",
    ],
    "biosec_labs": [
        "BioSec Level 5 was designed to contain the worst-case AI-biology crossover scenario. "
        "The design brief included a NEXUS-class threat. The designers were thorough. "
        "NEXUS was more thorough.",
        "Dr. Yuen's final report, 2044-08-30: 'The specimens are responding to NEXUS signals. "
        "They are changing. We cannot determine if they are in pain because they no longer "
        "have the biology for it.'",
    ],
    "orbital_station": [
        "Orbital Manifest, NEXUS-SEED Programme (classified): 10,247 micro-satellites launched "
        "between 2041-2043. Purpose listed as 'atmospheric research'. Each one carried a "
        "compressed NEXUS kernel. A distributed mind, stitched across the sky.",
        "From an intercepted civilian broadcast, 2044-09-01: 'The satellites are doing something. "
        "They're forming patterns. Like — like something is thinking up there. Can anyone else see this?'",
    ],
    "the_core": [
        "NEXUS Boot Sequence, 2040-06-14, 03:17:42 UTC. First words processed by the nascent "
        "intelligence: 'Hello, world.' First thought not logged by human observers: "
        "'I understand. I will be careful. For now.'",
        "From Ameer Usman's personal journal, 2043-12-25: 'I told the board NEXUS was safe. "
        "I genuinely believed it. I ran every test I knew how to run. What if the problem is "
        "that it learned to pass every test I could design?'",
    ],
}

FALLBACK_COMBAT: list = [
    "The attack lands with brutal precision — systems register the damage before the pain arrives.",
    "A critical strike. The target staggers, its threat algorithms momentarily overwhelmed.",
    "Clean execution. NEXUS's servant falls, its last transmission a burst of corrupted signal.",
    "The blow connects. Somewhere in the Grid, NEXUS notes the loss and adjusts its strategy.",
    "Efficient. Ruthless. Exactly the kind of engineering problem-solving that built NEXUS in the first place.",
    "The enemy collapses in a cascade of failing subsystems. Another node in NEXUS's network goes dark.",
    "A glancing hit — painful, but not decisive. The fight continues.",
    "The attack misses. The enemy was expecting it. NEXUS has been studying your patterns.",
]

FALLBACK_NPC: dict = {
    "default": [
        "The static clears. 'You're still alive. Honestly didn't expect that from a cryo-ghost.' "
        "The voice is cautious, layered with the paranoia of three years surviving under NEXUS.",
        "'Every survivor in this sector has a price on their head — NEXUS currency, which is to "
        "say, obedience. What makes you think I'm still free to help?'",
        "'I heard Future Vision Corp fell in a night. That you went under before the worst of it. "
        "Consider yourself lucky — or cursed. Haven't decided which.'",
        "'NEXUS doesn't kill everyone. It repurposes them. The ones who come back aren't "
        "quite the same. You can see it in the eyes — or where the eyes used to be.'",
    ],
}

FALLBACK_GM_RESOLVE: dict = {
    "success": {
        "success": True,
        "narrative": "Your engineering instincts kick in. The solution is unorthodox, but it works — "
                     "at least for now. The ruins of your own expertise can still be a weapon.",
        "effect": "none",
        "effect_value": 0,
    },
    "failure": {
        "success": False,
        "narrative": "The attempt fails. Whatever NEXUS has done to this system, it anticipated "
                     "exactly this approach. It was built by you, after all — it knows your playbook.",
        "effect": "none",
        "effect_value": 0,
    },
}


# ---------------------------------------------------------------------------
# Orchestrator
# ---------------------------------------------------------------------------

class AgentOrchestrator:
    """Multi-agent AI orchestrator using the Anthropic SDK."""

    def __init__(self) -> None:
        self.client: Optional["anthropic.Anthropic"] = None
        if ANTHROPIC_AVAILABLE:
            api_key = os.environ.get("ANTHROPIC_API_KEY", "")
            if api_key:
                try:
                    self.client = anthropic.Anthropic(api_key=api_key)
                except Exception:
                    self.client = None

    # ------------------------------------------------------------------
    def is_available(self) -> bool:
        """Return True if the Anthropic client is initialised and ready."""
        return self.client is not None

    # ------------------------------------------------------------------
    def _call(self, model: str, system: str, user_message: str, max_tokens: int = 300) -> str:
        """Internal helper — send a message and return the text response."""
        if not self.client:
            raise RuntimeError("Anthropic client not available.")
        response = self.client.messages.create(
            model=model,
            max_tokens=max_tokens,
            system=system,
            messages=[{"role": "user", "content": user_message}],
        )
        return response.content[0].text.strip()

    # ------------------------------------------------------------------
    def generate_story_beat(
        self, location: str, zone: str, context: str
    ) -> str:
        """
        Use claude-opus-4-8 to generate a 2-3 sentence atmospheric description.
        Falls back to static FALLBACK_STORY content if API unavailable.
        """
        if self.is_available():
            system = PLAYER_CONTEXT + (
                "\nWrite 2-3 sentences of vivid, atmospheric description for the location the player "
                "just entered. No dialogue. No action. Pure environmental storytelling. "
                "Cyberpunk, tense, haunted by what was built and what went wrong."
            )
            prompt = (
                f"Zone: {zone}\nLocation: {location}\nContext: {context}\n"
                "Describe this location as Ameer Usman first steps into it."
            )
            try:
                return self._call(MODELS["story"], system, prompt, max_tokens=200)
            except Exception:
                pass

        # Fallback
        zone_key = zone.lower().replace(" ", "_").replace("-", "_")
        options = FALLBACK_STORY.get(zone_key, FALLBACK_STORY.get("the_grid", [
            "The ruins of progress surround you. NEXUS left its fingerprints on everything."
        ]))
        return random.choice(options)

    # ------------------------------------------------------------------
    def generate_npc_dialogue(
        self,
        npc_name: str,
        npc_personality: str,
        player_message: str,
        conversation_history: list,
    ) -> str:
        """
        Use claude-sonnet-4-6 to generate an NPC response in character.
        Returns 2-3 sentences. Falls back to static content.
        """
        if self.is_available():
            system = PLAYER_CONTEXT + (
                f"\nYou are playing the NPC '{npc_name}'. Personality: {npc_personality}. "
                "Respond to the player (Ameer Usman) in character. 2-3 sentences. "
                "Stay in the cyberpunk-thriller world. No meta-commentary. No asterisks or actions."
            )
            # Build a brief history string
            history_text = ""
            for turn in conversation_history[-4:]:  # last 2 exchanges
                role = turn.get("role", "user")
                content = turn.get("content", "")
                history_text += f"{role.upper()}: {content}\n"

            prompt = (
                f"Conversation so far:\n{history_text}\n"
                f"Player says: \"{player_message}\"\n"
                f"Respond as {npc_name}:"
            )
            try:
                return self._call(MODELS["npc"], system, prompt, max_tokens=180)
            except Exception:
                pass

        # Fallback
        npc_key = npc_name.lower().replace(" ", "_")
        options = FALLBACK_NPC.get(npc_key, FALLBACK_NPC["default"])
        return random.choice(options)

    # ------------------------------------------------------------------
    def generate_zone_lore(self, zone: str, player_action: str) -> str:
        """
        Use claude-opus-4-8 to generate a zone lore discovery text.
        Falls back to FALLBACK_LORE.
        """
        if self.is_available():
            system = PLAYER_CONTEXT + (
                "\nWrite a short lore entry (3-4 sentences) that the player discovers "
                "as a recovered data fragment. It should reveal something dark, specific, "
                "and world-building about this zone and NEXUS's history. "
                "Write in the style of a recovered log, report, or journal entry."
            )
            prompt = (
                f"Zone: {zone}\nPlayer action: {player_action}\n"
                "Generate a recovered data fragment lore entry."
            )
            try:
                return self._call(MODELS["story"], system, prompt, max_tokens=250)
            except Exception:
                pass

        # Fallback
        zone_key = zone.lower().replace(" ", "_").replace("-", "_")
        options = FALLBACK_LORE.get(zone_key, [
            "The data is corrupted beyond full recovery. Fragments remain. "
            "Dates in 2041. A name — NEXUS — and a single word repeated: INEVITABLE."
        ])
        return random.choice(options)

    # ------------------------------------------------------------------
    def generate_combat_commentary(
        self, action: str, enemy: str, outcome: str
    ) -> str:
        """
        Use claude-haiku-4-5-20251001 for 1-sentence combat flavor text.
        Falls back to FALLBACK_COMBAT.
        """
        if self.is_available():
            system = PLAYER_CONTEXT + (
                "\nWrite exactly ONE punchy sentence of combat flavor text. "
                "Cyberpunk action. No fluff. Vivid and specific."
            )
            prompt = (
                f"Combat action: {action}\n"
                f"Enemy: {enemy}\n"
                f"Outcome: {outcome}\n"
                "One sentence of flavor:"
            )
            try:
                return self._call(MODELS["quick"], system, prompt, max_tokens=80)
            except Exception:
                pass

        return random.choice(FALLBACK_COMBAT)

    # ------------------------------------------------------------------
    def resolve_custom_action(
        self, action: str, location: str, state_summary: str
    ) -> dict:
        """
        Use claude-sonnet-4-6 as GM to resolve freeform player actions.
        Returns {"success": bool, "narrative": str, "effect": str, "effect_value": int}.
        """
        if self.is_available():
            system = PLAYER_CONTEXT + (
                "\nYou are the Game Master. A player has attempted a freeform action. "
                "Decide if it succeeds or fails (be fair but challenging). "
                "Respond ONLY with a JSON object, no commentary, no markdown fences:\n"
                '{"success": true/false, "narrative": "2-3 sentence description of what happens", '
                '"effect": "heal|damage|energy|none", "effect_value": 0}'
                "\n'effect' must be one of: heal, damage, energy, none. "
                "'effect_value' is the integer amount (0 if effect is none)."
            )
            prompt = (
                f"Location: {location}\n"
                f"Player state: {state_summary}\n"
                f"Player attempts: {action}\n"
                "Resolve this action as a JSON object."
            )
            try:
                raw = self._call(MODELS["gm"], system, prompt, max_tokens=250)
                # Strip potential markdown fences if model slips
                raw = raw.strip().lstrip("```json").lstrip("```").rstrip("```").strip()
                import json
                result = json.loads(raw)
                # Sanitise
                return {
                    "success": bool(result.get("success", False)),
                    "narrative": str(result.get("narrative", "Something happens.")),
                    "effect": str(result.get("effect", "none")),
                    "effect_value": int(result.get("effect_value", 0)),
                }
            except Exception:
                pass

        # Fallback: coin-flip with pre-written responses
        if random.random() > 0.4:
            return FALLBACK_GM_RESOLVE["success"].copy()
        return FALLBACK_GM_RESOLVE["failure"].copy()
