import random
from dataclasses import dataclass, field
from typing import List
from game.player import Player, SKILL_DEFINITIONS
from game.inventory import Inventory, ITEM_DEFINITIONS


@dataclass
class CombatState:
    enemy_hp: int = 0
    enemy_max_hp: int = 0
    enemy_name: str = ""
    enemy_attack: int = 0
    enemy_defense: int = 0
    enemy_xp: int = 0
    enemy_loot: List[str] = field(default_factory=list)
    enemy_special: str = ""
    enemy_special_cooldown: int = 3
    enemy_special_timer: int = 0

    player_stealth_turns: int = 0
    player_shield_active: bool = False
    player_stunned_turns: int = 0
    enemy_stunned_turns: int = 0

    turn: int = 0
    combat_log: List[str] = field(default_factory=list)
    is_over: bool = False
    player_won: bool = False


class CombatSystem:
    def __init__(self, player: Player, inventory: Inventory):
        self.player = player
        self.inventory = inventory

    def start_combat(self, enemy_data: dict) -> CombatState:
        state = CombatState(
            enemy_hp=enemy_data["hp"],
            enemy_max_hp=enemy_data["max_hp"],
            enemy_name=enemy_data["name"],
            enemy_attack=enemy_data["attack_power"],
            enemy_defense=enemy_data["defense"],
            enemy_xp=enemy_data["xp_reward"],
            enemy_loot=enemy_data.get("loot_table", []),
            enemy_special=enemy_data.get("special_ability", ""),
            enemy_special_cooldown=enemy_data.get("special_cooldown", 3),
        )
        return state

    def player_attack(self, combat: CombatState) -> str:
        """Basic attack. Returns narrative string."""
        base = random.randint(8, 18)
        crit = random.random() < 0.15
        damage = int(base * 1.5) if crit else base
        actual = max(1, damage - combat.enemy_defense // 2)
        combat.enemy_hp = max(0, combat.enemy_hp - actual)

        msg = f"You hack {combat.enemy_name} for [bold red]{actual}[/bold red] damage"
        if crit:
            msg += " [bold yellow](CRITICAL HIT!)[/bold yellow]"
        return msg

    def player_use_skill(self, skill_id: str, combat: CombatState) -> str:
        """Use a skill. Returns narrative string."""
        skill = SKILL_DEFINITIONS.get(skill_id, {})
        if not skill:
            return "Unknown skill."

        success, msg = self.player.use_skill(skill_id)
        if not success:
            return msg

        stype = skill["type"]

        if stype == "attack":
            lo, hi = skill["damage_range"]
            damage = random.randint(lo, hi)
            actual = max(1, damage - combat.enemy_defense // 3)
            combat.enemy_hp = max(0, combat.enemy_hp - actual)
            return (
                f"[bold cyan]{skill['name']}![/bold cyan] "
                f"Dealt [bold red]{actual}[/bold red] damage!"
            )

        elif stype == "buff":
            combat.player_stealth_turns = skill["duration"]
            return (
                f"[bold cyan]{skill['name']}![/bold cyan] "
                f"Stealth mode active for {skill['duration']} turns."
            )

        elif stype == "heal":
            lo, hi = skill["heal_range"]
            amount = random.randint(lo, hi)
            healed = self.player.heal(amount)
            return (
                f"[bold cyan]{skill['name']}![/bold cyan] "
                f"Restored [bold green]{healed}[/bold green] HP."
            )

        elif stype == "debuff":
            combat.enemy_stunned_turns = skill["stun_duration"]
            return (
                f"[bold cyan]{skill['name']}![/bold cyan] "
                f"{combat.enemy_name} is stunned for {skill['stun_duration']} turns!"
            )

        elif stype == "defend":
            combat.player_shield_active = True
            return (
                f"[bold cyan]{skill['name']}![/bold cyan] "
                f"Firewall shield deployed. Next attack blocked!"
            )

        return "Skill activated."

    def enemy_attack(self, combat: CombatState) -> str:
        """Enemy attacks player. Returns narrative string."""
        if combat.enemy_stunned_turns > 0:
            combat.enemy_stunned_turns -= 1
            return f"{combat.enemy_name} is stunned and cannot act!"

        # Check if special attack fires
        combat.enemy_special_timer += 1
        use_special = (
            combat.enemy_special
            and combat.enemy_special_timer >= combat.enemy_special_cooldown
            and random.random() < 0.4
        )

        if use_special:
            combat.enemy_special_timer = 0
            return self._enemy_special(combat)

        # Regular attack
        base_damage = random.randint(
            max(1, combat.enemy_attack - 3),
            combat.enemy_attack + 3,
        )

        # Stealth miss chance
        if combat.player_stealth_turns > 0:
            combat.player_stealth_turns -= 1
            if random.random() < 0.5:
                return f"{combat.enemy_name} attacks but misses! [Ghost Protocol active]"

        # Shield block
        if combat.player_shield_active:
            combat.player_shield_active = False
            return f"{combat.enemy_name} attacks — BLOCKED by Firewall Shield!"

        actual = self.player.take_damage(base_damage)
        return f"{combat.enemy_name} hits you for [bold red]{actual}[/bold red] damage!"

    def _enemy_special(self, combat: CombatState) -> str:
        special = combat.enemy_special
        if special == "Reality Hack":
            # Disable skills for 2 turns (tracked as a story event, handled by UI)
            damage = random.randint(20, 35)
            actual = self.player.take_damage(damage)
            return (
                f"[bold magenta]NEXUS PRIME — REALITY HACK![/bold magenta] "
                f"Your skills are disrupted! Took {actual} damage!"
            )
        else:
            # Generic special: 1.5x damage
            damage = int(combat.enemy_attack * 1.5)
            actual = self.player.take_damage(damage)
            return (
                f"[bold red]{combat.enemy_name} — "
                f"{special or 'SPECIAL ATTACK'}![/bold red] {actual} damage!"
            )

    def check_combat_end(self, combat: CombatState) -> bool:
        if combat.enemy_hp <= 0:
            combat.is_over = True
            combat.player_won = True
            return True
        if not self.player.is_alive():
            combat.is_over = True
            combat.player_won = False
            return True
        return False

    def get_loot(self, combat: CombatState) -> List[str]:
        """Determine what drops from enemy. Returns list of item_ids."""
        dropped = []
        for item_id in combat.enemy_loot:
            if random.random() < 0.4:
                dropped.append(item_id)
        return dropped

    def energy_regen(self, amount: int = 5) -> None:
        """Regen a small amount of energy each turn."""
        self.player.restore_energy(amount)
