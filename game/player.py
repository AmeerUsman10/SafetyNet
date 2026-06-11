from typing import Optional, Tuple
from game.state import GameState

SKILL_DEFINITIONS = {
    "sudo_blast": {
        "name": "SUDO BLAST",
        "description": "Overwhelming hack attack. Deals 35-50 damage.",
        "energy_cost": 30,
        "damage_range": (35, 50),
        "type": "attack",
    },
    "ghost_protocol": {
        "name": "GHOST PROTOCOL",
        "description": "Enter stealth mode. Enemies have 50% miss chance for 3 turns.",
        "energy_cost": 20,
        "duration": 3,
        "type": "buff",
    },
    "neural_patch": {
        "name": "NEURAL PATCH",
        "description": "Emergency self-repair. Restores 30-40 HP.",
        "energy_cost": 25,
        "heal_range": (30, 40),
        "type": "heal",
    },
    "recursive_loop": {
        "name": "RECURSIVE LOOP",
        "description": "Trap enemy in infinite loop. Stun for 2 turns.",
        "energy_cost": 40,
        "stun_duration": 2,
        "type": "debuff",
    },
    "firewall_shield": {
        "name": "FIREWALL SHIELD",
        "description": "Deploy personal firewall. Blocks the next attack completely.",
        "energy_cost": 15,
        "type": "defend",
    },
}


class Player:
    def __init__(self, state: GameState):
        self.state = state

    # Properties that proxy to state
    @property
    def hp(self): return self.state.player_hp

    @property
    def max_hp(self): return self.state.player_max_hp

    @property
    def energy(self): return self.state.player_energy

    @property
    def max_energy(self): return self.state.player_max_energy

    @property
    def level(self): return self.state.player_level

    @property
    def xp(self): return self.state.player_xp

    @property
    def skills(self): return self.state.skills

    def take_damage(self, amount: int) -> int:
        """Returns actual damage taken"""
        actual = max(0, amount)
        self.state.player_hp = max(0, self.state.player_hp - actual)
        return actual

    def heal(self, amount: int) -> int:
        """Returns actual amount healed"""
        before = self.state.player_hp
        self.state.player_hp = min(self.state.player_max_hp, self.state.player_hp + amount)
        return self.state.player_hp - before

    def spend_energy(self, amount: int) -> bool:
        """Returns False if not enough energy"""
        if self.state.player_energy < amount:
            return False
        self.state.player_energy -= amount
        return True

    def restore_energy(self, amount: int) -> int:
        before = self.state.player_energy
        self.state.player_energy = min(
            self.state.player_max_energy, self.state.player_energy + amount
        )
        return self.state.player_energy - before

    def is_alive(self) -> bool:
        return self.state.player_hp > 0

    def get_skill_info(self, skill_id: str) -> dict:
        return SKILL_DEFINITIONS.get(skill_id, {})

    def use_skill(self, skill_id: str) -> Tuple[bool, str]:
        """Returns (success, message). Deducts energy."""
        if skill_id not in self.state.skills:
            return False, "You haven't unlocked this skill."
        skill = SKILL_DEFINITIONS.get(skill_id)
        if not skill:
            return False, "Unknown skill."
        if not self.spend_energy(skill["energy_cost"]):
            return False, f"Not enough energy. Need {skill['energy_cost']}."
        return True, f"Used {skill['name']}!"
