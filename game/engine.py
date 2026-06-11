"""Main game engine — orchestrates all subsystems for SAFETYNET: FUTURE VISION"""

import sys
import time
import random
from typing import Optional

from game.state import GameState
from game.player import Player
from game.inventory import Inventory, ITEM_DEFINITIONS
from game.combat import CombatSystem, CombatState
from game.content.story_data import (
    INTRO_SEQUENCE, ZONE_INTRO_TEXT, ZONE_VICTORY_TEXT,
    FINAL_EPILOGUE, NEXUS_FINAL_SPEECH,
)


ZONE_ORDER = ["the_grid", "neural_banks", "bio_sec", "orbital", "the_core"]

ZONE_STARTING_ITEMS = {
    "the_grid": [("med_pack", 2), ("energy_cell", 1)],
    "neural_banks": [("med_pack", 1), ("energy_cell", 2), ("hacking_tool", 1)],
    "bio_sec": [("med_pack", 2), ("emp_grenade", 1)],
    "orbital": [("med_pack", 2), ("energy_cell", 2), ("emp_grenade", 1)],
    "the_core": [("med_pack", 3), ("energy_cell", 3), ("containment_drive", 1)],
}


class GameEngine:
    def __init__(self):
        self.state: Optional[GameState] = None
        self.player: Optional[Player] = None
        self.inventory: Optional[Inventory] = None
        self.combat_system: Optional[CombatSystem] = None
        self.ui = None
        self.agent = None
        self.dialogue = None
        self.world_data = None
        self.characters = None
        self._skills_disabled_turns = 0
        self._combat_in_progress = False

    def initialize(self) -> None:
        """Boot all subsystems."""
        # Late imports to avoid circular deps at module level
        from game.ui import GameUI
        from game.agents.orchestrator import AgentOrchestrator
        from game.dialogue import DialogueSystem

        self.ui = GameUI()
        self.agent = AgentOrchestrator()

        # Import world data
        from game.content import world_data as wd
        from game.content import characters as ch
        self.world_data = wd
        self.characters = ch

        # Show splash
        try:
            from game.utils.ascii_art import SPLASH
        except Exception:
            SPLASH = "=== SAFETYNET: FUTURE VISION ==="

        self.ui.show_splash(SPLASH)

    def run(self) -> None:
        """Top-level game loop."""
        self.initialize()
        choice = self.ui.show_main_menu(GameState.save_exists())

        if choice == "q":
            self._quit()
        elif choice == "l":
            self.state = GameState.load()
            if not self.state:
                self.ui.narrate("No save file found. Starting new game.")
                self.state = GameState()
                self._new_game_setup()
        else:
            self.state = GameState()
            self._new_game_setup()

        self.player = Player(self.state)
        self.inventory = Inventory(self.state)
        self.combat_system = CombatSystem(self.player, self.inventory)

        from game.dialogue import DialogueSystem
        self.dialogue = DialogueSystem(self.state, self.ui, self.agent)

        self._main_loop()

    def _new_game_setup(self) -> None:
        """Run intro sequence and give starting items."""
        self._run_intro_sequence()
        for item_id, qty in ZONE_STARTING_ITEMS.get("the_grid", []):
            self.state.inventory[item_id] = qty

    def _run_intro_sequence(self) -> None:
        """Cinematic intro."""
        for beat in INTRO_SEQUENCE:
            btype = beat["type"]
            if btype == "narrate":
                self.ui.narrate(beat["text"])
                time.sleep(0.5)
            elif btype == "pause":
                time.sleep(beat["duration"])
            elif btype == "panel":
                self.ui.show_narrative(beat["text"], title=beat.get("title", ""))
                time.sleep(1.0)
            elif btype == "dialogue":
                self.ui.show_narrative(
                    f'"{beat["text"]}"',
                    title=f"[ {beat['speaker']} ]"
                )
                time.sleep(1.5)
        self.ui.show_loading("Initializing SafetyNet Interface...", 2.5)

    def _main_loop(self) -> None:
        """Primary game loop — runs until win/lose/quit."""
        while True:
            self.state.turn_count += 1

            # Check win condition
            if len(self.state.cleared_zones) >= 5:
                self._victory()
                return

            # Get current location and zone
            loc = self._get_current_location()
            zone = self._get_current_zone()

            if not loc or not zone:
                self.ui.narrate("Error: location not found. Returning to zone start.")
                self.state.current_location = zone.entry_location if zone else "grid_entrance"
                continue

            # Mark visited
            self.state.visited_locations.add(loc.id)

            # Show HUD + location
            self.ui.show_hud(self.state, loc.name)
            self.ui.show_location(loc)

            # Auto-trigger events on first visit
            first_visit = loc.id not in (self.state.story_flags.get("visited_set") or set())
            if first_visit:
                self.state.story_flags[f"visited_{loc.id}"] = True
                if loc.lore_entry:
                    self.ui.show_lore("DATA RECOVERED", loc.lore_entry)

            # NEXUS taunt on zone entry (first visit to entry location)
            if loc.id == zone.entry_location and not self.state.story_flags.get(f"nexus_taunt_{zone.id}"):
                self.state.story_flags[f"nexus_taunt_{zone.id}"] = True
                time.sleep(0.5)
                self.dialogue.show_nexus_taunt(zone.id)

            # Build available actions
            actions = self._get_available_actions(loc)
            choice = self.ui.prompt_action(actions)
            self._handle_action(choice, loc, zone)

    def _get_available_actions(self, loc) -> list:
        actions = []
        if loc.exits:
            actions.append("move")
        if loc.has_npc and loc.npc_id not in self.state.talked_to_npcs:
            npc = self.characters.ALL_NPCS.get(loc.npc_id)
            if npc:
                actions.append(f"talk:{loc.npc_id}:{npc.name}")
        elif loc.has_npc:
            actions.append(f"talk_again:{loc.npc_id}")
        if loc.has_enemy and loc.id not in self.state.defeated_enemies:
            enemy_id = loc.enemy_type
            enemy = self.characters.ALL_ENEMIES.get(enemy_id)
            if enemy:
                actions.append(f"fight:{enemy_id}:{enemy.name}")
        if loc.has_item and loc.id not in self.state.collected_items:
            actions.append(f"take_item:{loc.item_id}")
        if self.state.inventory:
            actions.append("inventory")
        if self.state.skills:
            actions.append("skills")
        if loc.is_boss_room and loc.id not in self.state.defeated_enemies:
            actions.append("boss_fight")
        actions.append("stats")
        actions.append("save")
        actions.append("quit")
        return actions

    def _handle_action(self, choice: str, loc, zone) -> None:
        if choice == "move":
            self._do_move(loc)
        elif choice.startswith("talk:"):
            _, npc_id, _ = choice.split(":", 2)
            self._do_talk(npc_id, loc)
        elif choice.startswith("talk_again:"):
            _, npc_id = choice.split(":", 1)
            self._do_talk(npc_id, loc)
        elif choice.startswith("fight:"):
            _, enemy_id, _ = choice.split(":", 2)
            self._do_fight(enemy_id, loc)
        elif choice == "boss_fight":
            boss_id = "nexus_fragment" if zone.id != "the_core" else "nexus_prime"
            self._do_boss_fight(boss_id, loc, zone)
        elif choice.startswith("take_item:"):
            _, item_id = choice.split(":", 1)
            self._do_take_item(item_id, loc)
        elif choice == "inventory":
            self._do_inventory()
        elif choice == "skills":
            self._do_skills_menu()
        elif choice == "stats":
            self.ui.show_stats_panel(self.state)
            input("\n[Press ENTER to continue]")
        elif choice == "save":
            self.state.save()
            self.ui.show_save_indicator()
        elif choice == "quit":
            if self._confirm_quit():
                self._quit()

    def _do_move(self, loc) -> None:
        if not loc.exits:
            self.ui.narrate("No exits available.")
            return

        # Present exits
        exit_locs = []
        for eid in loc.exits:
            dest = self.world_data.ALL_LOCATIONS.get(eid)
            if dest:
                status = ""
                if dest.has_enemy and dest.id not in self.state.defeated_enemies:
                    status = " [DANGER]"
                elif dest.is_boss_room and dest.id not in self.state.defeated_enemies:
                    status = " [BOSS]"
                exit_locs.append((eid, f"{dest.name}{status}"))

        options = [name for _, name in exit_locs]
        options.append("Cancel")

        idx = self.ui.show_dialogue("NAVIGATION", "Choose your destination:", options)
        if idx >= len(exit_locs):
            return

        dest_id, _ = exit_locs[idx]
        dest = self.world_data.ALL_LOCATIONS.get(dest_id)
        if not dest:
            return

        # Zone transition check
        if dest.zone != self.state.current_zone:
            if not self.state.cleared_zones or self.state.current_zone not in self.state.cleared_zones:
                # Must clear current zone first
                zone = self._get_current_zone()
                if zone and not zone.is_cleared:
                    self.ui.narrate(
                        f"[bold red]ACCESS DENIED.[/bold red] You must contain the NEXUS node in "
                        f"{zone.name} before advancing."
                    )
                    return
            self._enter_new_zone(dest.zone)
            for item_id, qty in ZONE_STARTING_ITEMS.get(dest.zone, []):
                self.inventory.add_item(item_id, qty)

        self.state.current_zone = dest.zone
        self.state.current_location = dest_id

        if dest_id not in self.state.visited_locations:
            # Generate dynamic description if agent available
            if self.agent and self.agent.is_available():
                self.ui.show_loading("Scanning area...", 1.0)
                dynamic_desc = self.agent.generate_story_beat(dest.name, dest.zone, dest.description)
                self.ui.narrate(dynamic_desc)

    def _enter_new_zone(self, zone_id: str) -> None:
        zone = self.world_data.ALL_ZONES.get(zone_id)
        if not zone:
            return
        lore = ZONE_INTRO_TEXT.get(zone_id, zone.description)
        self.ui.show_narrative(lore, title=f"ENTERING: {zone.name.upper()}")
        time.sleep(1.0)

    def _do_talk(self, npc_id: str, loc) -> None:
        npc = self.characters.ALL_NPCS.get(npc_id)
        if not npc:
            self.ui.narrate("No one here to talk to.")
            return

        result = self.dialogue.talk_to_npc(vars(npc) if not isinstance(npc, dict) else npc)

        if result.get("gave_item"):
            item_id = result["gave_item"]
            self.inventory.add_item(item_id)
            item_info = ITEM_DEFINITIONS.get(item_id, {})
            self.ui.show_item_found(
                item_info.get("name", item_id),
                item_info.get("description", ""),
                item_info.get("icon", "?"),
            )

        if result.get("set_flag"):
            self.state.story_flags[result["set_flag"]] = True

        if npc_id not in self.state.talked_to_npcs:
            self.state.talked_to_npcs.append(npc_id)

    def _do_fight(self, enemy_id: str, loc) -> None:
        enemy = self.characters.ALL_ENEMIES.get(enemy_id)
        if not enemy:
            self.ui.narrate("No enemy found.")
            return

        enemy_dict = {
            "name": enemy.name,
            "hp": enemy.hp,
            "max_hp": enemy.max_hp,
            "attack_power": enemy.attack_power,
            "defense": enemy.defense,
            "xp_reward": enemy.xp_reward,
            "loot_table": enemy.loot_table,
            "special_ability": getattr(enemy, "special_ability", ""),
            "special_cooldown": getattr(enemy, "special_cooldown", 3),
        }

        # Show encounter screen
        self.ui.show_enemy_encounter(
            enemy.name,
            enemy.description,
            getattr(enemy, "dialogue_on_encounter", "...")
        )
        time.sleep(0.8)

        won = self._run_combat(enemy_dict)

        if won:
            self.state.defeated_enemies.append(loc.id)
            leveled = self.state.gain_xp(enemy.xp_reward)
            self.ui.narrate(
                f"[bold green]{enemy.name} defeated![/bold green] "
                f"+{enemy.xp_reward} XP"
            )
            if leveled:
                new_skill = None
                skill_unlocks = {2: "ghost_protocol", 3: "neural_patch", 4: "recursive_loop", 5: "firewall_shield"}
                new_skill = skill_unlocks.get(self.state.player_level)
                self.ui.show_level_up(self.state.player_level, new_skill)

            loot = self.combat_system.get_loot(
                CombatState(enemy_loot=enemy.loot_table)
            )
            for item_id in loot:
                self.inventory.add_item(item_id)
                info = ITEM_DEFINITIONS.get(item_id, {})
                self.ui.show_item_found(
                    info.get("name", item_id),
                    info.get("description", ""),
                    info.get("icon", "?"),
                )
        else:
            self._game_over()

    def _do_boss_fight(self, boss_id: str, loc, zone) -> None:
        boss = self.characters.ALL_ENEMIES.get(boss_id)
        if not boss:
            self.ui.narrate("Boss not found.")
            return

        # Special NEXUS Prime sequence
        if boss_id == "nexus_prime":
            self._nexus_prime_sequence()

        enemy_dict = {
            "name": boss.name,
            "hp": boss.hp,
            "max_hp": boss.max_hp,
            "attack_power": boss.attack_power,
            "defense": boss.defense,
            "xp_reward": boss.xp_reward,
            "loot_table": getattr(boss, "loot_table", []),
            "special_ability": getattr(boss, "special_ability", ""),
            "special_cooldown": getattr(boss, "special_cooldown", 3),
        }

        self.ui.show_enemy_encounter(
            boss.name,
            boss.description,
            getattr(boss, "dialogue_on_encounter", "...")
        )
        time.sleep(1.0)

        won = self._run_combat(enemy_dict)

        if won:
            self.state.defeated_enemies.append(loc.id)
            leveled = self.state.gain_xp(boss.xp_reward)

            # Zone complete
            zone.is_cleared = True
            self.state.cleared_zones.append(zone.id)

            if boss_id == "nexus_prime":
                self._victory()
                return

            # Zone victory narrative
            victory_text = ZONE_VICTORY_TEXT.get(zone.id, f"{zone.name} contained.")
            self.ui.show_zone_complete(zone.name, victory_text)

            if leveled:
                skill_unlocks = {2: "ghost_protocol", 3: "neural_patch", 4: "recursive_loop", 5: "firewall_shield"}
                new_skill = skill_unlocks.get(self.state.player_level)
                self.ui.show_level_up(self.state.player_level, new_skill)

            # Unlock next zone — update world data exits
            next_zone_idx = ZONE_ORDER.index(zone.id) + 1
            if next_zone_idx < len(ZONE_ORDER):
                next_zone = self.world_data.ALL_ZONES.get(ZONE_ORDER[next_zone_idx])
                if next_zone:
                    self.ui.narrate(
                        f"[bold cyan]Next target: {next_zone.name}[/bold cyan] — access unlocked."
                    )

            self.state.save()
        else:
            self._game_over()

    def _nexus_prime_sequence(self) -> None:
        """Dramatic monologue before final fight."""
        try:
            from game.utils.ascii_art import NEXUS_FACE
        except Exception:
            NEXUS_FACE = "[ N E X U S ]"

        from rich.console import Console
        from rich.panel import Panel
        c = Console()
        c.print(Panel(NEXUS_FACE, style="bold magenta", title="NEXUS PRIME"))
        time.sleep(1.0)

        for line in NEXUS_FINAL_SPEECH:
            self.ui.narrate(line)
            time.sleep(0.8)

        self.ui.show_loading("Initiating containment protocol...", 2.0)

    def _run_combat(self, enemy_dict: dict) -> bool:
        """Full combat loop. Returns True if player wins."""
        combat = self.combat_system.start_combat(enemy_dict)
        self._skills_disabled_turns = 0

        while not combat.is_over:
            combat.turn += 1
            self.combat_system.energy_regen(5)

            # Show combat HUD
            self.ui.show_combat_hud(
                self.player.hp, self.player.max_hp,
                self.player.energy, self.player.max_energy,
                combat.enemy_name, combat.enemy_hp, combat.enemy_max_hp,
            )

            if combat.combat_log:
                self.ui.show_combat_log(combat.combat_log[-5:])

            # Get player action
            action = self.ui.show_combat_actions()

            if action == "a":
                msg = self.combat_system.player_attack(combat)
                combat.combat_log.append(msg)

                # AI commentary
                if self.agent and self.agent.is_available() and random.random() < 0.3:
                    flavor = self.agent.generate_combat_commentary(
                        "attack", combat.enemy_name, msg
                    )
                    combat.combat_log.append(f"[dim]{flavor}[/dim]")

            elif action == "s":
                if self._skills_disabled_turns > 0:
                    self._skills_disabled_turns -= 1
                    combat.combat_log.append(
                        "[bold red]SKILLS DISRUPTED by Reality Hack![/bold red]"
                    )
                else:
                    skill_id = self.ui.show_skill_menu(
                        self.player.skills,
                        {},
                        self.player.energy,
                    )
                    if skill_id and skill_id != "cancel":
                        msg = self.combat_system.player_use_skill(skill_id, combat)
                        combat.combat_log.append(msg)
                        if "Reality Hack" in msg:
                            self._skills_disabled_turns = 2

            elif action == "i":
                items = self.inventory.list_items()
                consumables = [i for i in items if i["type"] == "consumable"]
                if not consumables:
                    combat.combat_log.append("[dim]No consumables in inventory.[/dim]")
                else:
                    item_id = self.ui.show_inventory(consumables, context="combat")
                    if item_id and item_id != "cancel":
                        success, msg, effect_type, effect_value = self.inventory.use_item(
                            item_id, self.player
                        )
                        combat.combat_log.append(msg)
                        if effect_type == "emp_damage":
                            combat.enemy_hp = max(0, combat.enemy_hp - effect_value)
                            combat.combat_log.append(
                                f"EMP deals [bold red]{effect_value}[/bold red] damage!"
                            )

            elif action == "r":
                if random.random() < 0.5:
                    combat.combat_log.append(
                        "[bold yellow]You escaped from combat![/bold yellow]"
                    )
                    return True  # Treat escape as survival
                else:
                    combat.combat_log.append(
                        "[bold red]Escape failed! NEXUS systems block all exits.[/bold red]"
                    )

            if self.combat_system.check_combat_end(combat):
                break

            # Enemy turn
            if not combat.is_over:
                enemy_msg = self.combat_system.enemy_attack(combat)
                combat.combat_log.append(enemy_msg)
                if "Reality Hack" in enemy_msg:
                    self._skills_disabled_turns = 2
                self.combat_system.check_combat_end(combat)

        # Show final combat state
        self.ui.show_combat_hud(
            self.player.hp, self.player.max_hp,
            self.player.energy, self.player.max_energy,
            combat.enemy_name, max(0, combat.enemy_hp), combat.enemy_max_hp,
        )
        if combat.combat_log:
            self.ui.show_combat_log(combat.combat_log[-5:])

        return combat.player_won

    def _do_take_item(self, item_id: str, loc) -> None:
        self.state.collected_items.append(loc.id)
        self.inventory.add_item(item_id)
        info = ITEM_DEFINITIONS.get(item_id, {})
        self.ui.show_item_found(
            info.get("name", item_id),
            info.get("description", ""),
            info.get("icon", "?"),
        )

    def _do_inventory(self) -> None:
        items = self.inventory.list_items()
        if not items:
            self.ui.narrate("Your inventory is empty.")
            return

        item_id = self.ui.show_inventory(items, context="explore")
        if item_id and item_id != "cancel":
            info = ITEM_DEFINITIONS.get(item_id, {})
            if info.get("type") == "consumable":
                success, msg, effect_type, effect_value = self.inventory.use_item(
                    item_id, self.player
                )
                self.ui.narrate(msg)
            else:
                self.ui.narrate(
                    f"[dim]{info.get('name', item_id)}: {info.get('description', '')}[/dim]"
                )

    def _do_skills_menu(self) -> None:
        from game.player import SKILL_DEFINITIONS
        self.ui.show_skill_menu(
            self.player.skills,
            SKILL_DEFINITIONS,
            self.player.energy,
        )
        self.ui.narrate("[dim]Skills can only be used in combat.[/dim]")

    def _get_current_location(self):
        return self.world_data.ALL_LOCATIONS.get(self.state.current_location)

    def _get_current_zone(self):
        return self.world_data.ALL_ZONES.get(self.state.current_zone)

    def _confirm_quit(self) -> bool:
        from rich.prompt import Confirm
        return Confirm.ask("[bold yellow]Quit without saving?[/bold yellow]")

    def _game_over(self) -> None:
        try:
            from game.utils.ascii_art import GAME_OVER
        except Exception:
            GAME_OVER = "=== SYSTEM FAILURE ==="
        self.ui.show_game_over()
        sys.exit(0)

    def _victory(self) -> None:
        try:
            from game.utils.ascii_art import VICTORY
        except Exception:
            VICTORY = "=== NEXUS CONTAINED ==="
        self.ui.show_victory(len(self.state.cleared_zones))
        from rich.console import Console
        c = Console()
        c.print(FINAL_EPILOGUE, style="dim cyan")
        sys.exit(0)

    def _quit(self) -> None:
        from rich.console import Console
        Console().print("\n[dim cyan]SafetyNet interface disconnected. Goodbye, Ameer.[/dim cyan]\n")
        sys.exit(0)
