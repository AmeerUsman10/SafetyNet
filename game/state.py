import json, os
from dataclasses import dataclass, field, asdict
from typing import Dict, List, Optional, Set
from datetime import datetime

SAVE_FILE = "save_game.json"

@dataclass
class GameState:
    # Player
    player_hp: int = 100
    player_max_hp: int = 100
    player_energy: int = 100
    player_max_energy: int = 100
    player_level: int = 1
    player_xp: int = 0
    player_xp_to_next: int = 100

    # Stats
    intelligence: int = 10
    stealth: int = 8
    hacking: int = 12

    # Inventory: {item_id: quantity}
    inventory: Dict[str, int] = field(default_factory=dict)

    # Skills unlocked
    skills: List[str] = field(default_factory=lambda: ["sudo_blast"])

    # World state
    current_zone: str = "the_grid"
    current_location: str = "grid_entrance"
    visited_locations: Set[str] = field(default_factory=set)
    cleared_zones: List[str] = field(default_factory=list)
    defeated_enemies: List[str] = field(default_factory=list)
    talked_to_npcs: List[str] = field(default_factory=list)
    collected_items: List[str] = field(default_factory=list)

    # Story flags
    story_flags: Dict[str, bool] = field(default_factory=dict)

    # Meta
    turn_count: int = 0
    save_timestamp: str = ""

    def save(self) -> None:
        self.save_timestamp = datetime.now().isoformat()
        data = asdict(self)
        data["visited_locations"] = list(self.visited_locations)
        with open(SAVE_FILE, "w") as f:
            json.dump(data, f, indent=2)

    @classmethod
    def load(cls) -> Optional["GameState"]:
        if not os.path.exists(SAVE_FILE):
            return None
        with open(SAVE_FILE) as f:
            data = json.load(f)
        data["visited_locations"] = set(data.get("visited_locations", []))
        return cls(**data)

    @classmethod
    def save_exists(cls) -> bool:
        return os.path.exists(SAVE_FILE)

    def gain_xp(self, amount: int) -> bool:
        """Returns True if leveled up"""
        self.player_xp += amount
        if self.player_xp >= self.player_xp_to_next:
            self.level_up()
            return True
        return False

    def level_up(self) -> None:
        self.player_level += 1
        self.player_xp -= self.player_xp_to_next
        self.player_xp_to_next = int(self.player_xp_to_next * 1.5)
        self.player_max_hp += 15
        self.player_hp = self.player_max_hp
        self.player_max_energy += 10
        self.player_energy = self.player_max_energy
        self.intelligence += 1
        self.stealth += 1
        self.hacking += 2
        # Unlock skills at certain levels
        skill_unlocks = {
            2: "ghost_protocol",
            3: "neural_patch",
            4: "recursive_loop",
            5: "firewall_shield",
        }
        if self.player_level in skill_unlocks:
            skill = skill_unlocks[self.player_level]
            if skill not in self.skills:
                self.skills.append(skill)
