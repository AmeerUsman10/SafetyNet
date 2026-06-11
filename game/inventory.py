from typing import Dict, List, Optional
from game.state import GameState

ITEM_DEFINITIONS = {
    "med_pack": {
        "name": "Med-Pack",
        "description": "Emergency medical nanobots. Restores 40 HP.",
        "type": "consumable",
        "effect": "heal",
        "value": 40,
        "icon": "💊",
    },
    "energy_cell": {
        "name": "Energy Cell",
        "description": "Compact power source. Restores 35 energy.",
        "type": "consumable",
        "effect": "energy",
        "value": 35,
        "icon": "⚡",
    },
    "emp_grenade": {
        "name": "EMP Grenade",
        "description": "Electromagnetic pulse device. Deals 25 damage to mechanical enemies.",
        "type": "consumable",
        "effect": "emp_damage",
        "value": 25,
        "icon": "💣",
    },
    "hacking_tool": {
        "name": "Hacking Tool",
        "description": "Specialized intrusion toolkit. +5 to hacking for one encounter.",
        "type": "consumable",
        "effect": "hack_boost",
        "value": 5,
        "icon": "🔧",
    },
    "decryption_key": {
        "name": "Decryption Key",
        "description": "Encrypted access token. Unlocks NEXUS-secured doors.",
        "type": "key",
        "effect": "unlock",
        "value": 0,
        "icon": "🔑",
    },
    "nexus_shard": {
        "name": "NEXUS Core Shard",
        "description": "A fragment of NEXUS's consciousness. Pulsates with alien intelligence.",
        "type": "quest",
        "effect": "story",
        "value": 0,
        "icon": "💎",
    },
    "containment_drive": {
        "name": "Containment Drive",
        "description": "The device you built to trap NEXUS. Nearly full.",
        "type": "quest",
        "effect": "story",
        "value": 0,
        "icon": "📀",
    },
}


class Inventory:
    def __init__(self, state: GameState):
        self.state = state

    def add_item(self, item_id: str, quantity: int = 1) -> bool:
        if item_id not in ITEM_DEFINITIONS:
            return False
        self.state.inventory[item_id] = self.state.inventory.get(item_id, 0) + quantity
        return True

    def remove_item(self, item_id: str, quantity: int = 1) -> bool:
        current = self.state.inventory.get(item_id, 0)
        if current < quantity:
            return False
        self.state.inventory[item_id] = current - quantity
        if self.state.inventory[item_id] == 0:
            del self.state.inventory[item_id]
        return True

    def has_item(self, item_id: str, quantity: int = 1) -> bool:
        return self.state.inventory.get(item_id, 0) >= quantity

    def get_item_info(self, item_id: str) -> Optional[dict]:
        return ITEM_DEFINITIONS.get(item_id)

    def list_items(self) -> list:
        result = []
        for item_id, qty in self.state.inventory.items():
            info = ITEM_DEFINITIONS.get(item_id, {})
            result.append({
                "id": item_id,
                "name": info.get("name", item_id),
                "quantity": qty,
                "description": info.get("description", ""),
                "type": info.get("type", ""),
                "icon": info.get("icon", "?"),
            })
        return result

    def use_item(self, item_id: str, player) -> tuple:
        """Returns (success: bool, message: str, effect_type: str, effect_value: int)"""
        info = ITEM_DEFINITIONS.get(item_id)
        if not info:
            return False, "Unknown item.", "", 0
        if not self.has_item(item_id):
            return False, "You don't have that item.", "", 0
        if info["type"] not in ("consumable",):
            return False, f"The {info['name']} can't be used directly.", "", 0

        self.remove_item(item_id)
        effect = info["effect"]
        value = info["value"]

        if effect == "heal":
            healed = player.heal(value)
            return True, f"Used {info['name']}. Restored {healed} HP.", "heal", healed
        elif effect == "energy":
            restored = player.restore_energy(value)
            return True, f"Used {info['name']}. Restored {restored} energy.", "energy", restored
        elif effect in ("emp_damage", "hack_boost"):
            return True, f"Used {info['name']}.", effect, value

        return False, "That item can't be used here.", "", 0
