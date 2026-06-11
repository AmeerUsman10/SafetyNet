import os
import time
import random
from typing import Optional, List, Dict, Any
from rich.console import Console
from rich.panel import Panel
from rich.table import Table
from rich.columns import Columns
from rich.text import Text
from rich.layout import Layout
from rich.rule import Rule
from rich.prompt import Prompt, Confirm
from rich.progress import track
from rich import box
from rich.align import Align
from rich.style import Style
from rich.spinner import Spinner
from rich.live import Live

from game.state import GameState

console = Console()


class GameUI:
    """All terminal rendering for SAFETYNET."""

    COLORS = {
        "primary": "cyan",
        "danger": "red",
        "warning": "yellow",
        "success": "green",
        "muted": "dim white",
        "nexus": "magenta",
        "player": "bright_cyan",
    }

    # -------------------------------------------------------------------------
    # Internal helpers
    # -------------------------------------------------------------------------

    def _hp_bar(self, current: int, maximum: int, width: int = 20) -> Text:
        """Return a coloured HP bar as a Rich Text object."""
        if maximum <= 0:
            maximum = 1
        ratio = max(0, min(1, current / maximum))
        filled = int(ratio * width)
        empty = width - filled
        colour = "green" if ratio > 0.5 else ("yellow" if ratio > 0.25 else "red")
        bar = Text()
        bar.append("█" * filled, style=colour)
        bar.append("░" * empty, style="dim")
        bar.append(f" {current}/{maximum}", style="white")
        return bar

    def _energy_bar(self, current: int, maximum: int, width: int = 20) -> Text:
        """Return a coloured Energy bar as a Rich Text object."""
        if maximum <= 0:
            maximum = 1
        ratio = max(0, min(1, current / maximum))
        filled = int(ratio * width)
        empty = width - filled
        bar = Text()
        bar.append("█" * filled, style="cyan")
        bar.append("░" * empty, style="dim")
        bar.append(f" {current}/{maximum}", style="white")
        return bar

    def _clear(self) -> None:
        console.clear()

    # -------------------------------------------------------------------------
    # 1. Splash screen
    # -------------------------------------------------------------------------

    def show_splash(self, ascii_art: str) -> None:
        """Show splash screen with pulsing effect, press any key prompt."""
        self._clear()
        colours = ["cyan", "bright_cyan", "white", "bright_cyan", "cyan"]
        for colour in colours:
            self._clear()
            splash_text = Text(ascii_art, style=colour, justify="center")
            subtitle = Text(
                "\n  SAFETYNET: FUTURE VISION  \n  A Cyberpunk Survival RPG  ",
                style=f"bold {colour}",
                justify="center",
            )
            console.print(
                Panel(
                    Align.center(Text.assemble(splash_text, "\n", subtitle)),
                    border_style=colour,
                    box=box.DOUBLE,
                    padding=(1, 4),
                )
            )
            time.sleep(0.18)

        console.print()
        console.print(
            Align.center(
                Text("[ PRESS ENTER TO CONTINUE ]", style="blink bold cyan")
            )
        )
        input()

    # -------------------------------------------------------------------------
    # 2. Main menu
    # -------------------------------------------------------------------------

    def show_main_menu(self, has_save: bool) -> str:
        """Menu: (N)ew Game, (L)oad, (Q)uit. Returns choice."""
        self._clear()
        menu_items = [
            ("[N] New Game", "bold green"),
            ("[L] Load Game" if has_save else "[L] Load Game  (no save)", "bold cyan" if has_save else "dim"),
            ("[Q] Quit", "bold red"),
        ]
        menu_text = Text(justify="center")
        menu_text.append("SAFETYNET: FUTURE VISION\n\n", style="bold magenta")
        for label, style in menu_items:
            menu_text.append(f"  {label}\n", style=style)

        console.print()
        console.print(
            Panel(
                Align.center(menu_text),
                title="[bold cyan]MAIN MENU[/bold cyan]",
                border_style="cyan",
                box=box.DOUBLE,
                padding=(2, 6),
            )
        )
        console.print()
        while True:
            choice = Prompt.ask(
                "[cyan]>[/cyan] Choose",
                choices=["n", "l", "q", "N", "L", "Q"],
                show_choices=False,
            ).lower()
            if choice == "l" and not has_save:
                console.print("[red]No save file found.[/red]")
                continue
            return choice

    # -------------------------------------------------------------------------
    # 3. HUD bar
    # -------------------------------------------------------------------------

    def show_hud(self, state: GameState, location_name: str) -> None:
        """Top HUD bar: HP bar, Energy bar, Level, Zone name, Location name."""
        hp_bar = self._hp_bar(state.hp, state.max_hp, width=16)
        en_bar = self._energy_bar(state.energy, state.max_energy, width=16)

        hud = Table.grid(expand=True)
        hud.add_column(ratio=3)
        hud.add_column(ratio=3)
        hud.add_column(ratio=2)
        hud.add_column(ratio=2)
        hud.add_column(ratio=3)

        hp_cell = Text.assemble(("HP  ", "bold red"), hp_bar)
        en_cell = Text.assemble(("EN  ", "bold cyan"), en_bar)
        lvl_cell = Text(f"LVL {state.level}", style="bold yellow", justify="center")
        zone_cell = Text(f"Zone: {state.current_zone}", style="bold magenta", justify="center")
        loc_cell = Text(location_name, style="bold bright_cyan", justify="right")

        hud.add_row(hp_cell, en_cell, lvl_cell, zone_cell, loc_cell)

        console.print(
            Panel(hud, border_style="dim cyan", box=box.ROUNDED, padding=(0, 1))
        )

    # -------------------------------------------------------------------------
    # 4. Location display
    # -------------------------------------------------------------------------

    def show_location(self, location: Any) -> None:
        """Show current location as a Rich Panel with description and exits."""
        exits_text = Text()
        exits = getattr(location, "exits", {}) or {}
        if exits:
            exits_text.append("\nExits: ", style="bold yellow")
            exits_text.append(", ".join(exits.keys()), style="yellow")
        else:
            exits_text.append("\n[No exits visible]", style="dim")

        desc = getattr(location, "description", "An undefined place in the ruins.")
        name = getattr(location, "name", "Unknown Location")

        body = Text()
        body.append(desc + "\n", style="white")
        body.append_text(exits_text)

        console.print()
        console.print(
            Panel(
                body,
                title=f"[bold cyan]{name}[/bold cyan]",
                border_style="cyan",
                box=box.ROUNDED,
                padding=(1, 2),
            )
        )

    # -------------------------------------------------------------------------
    # 5. Zone map
    # -------------------------------------------------------------------------

    def show_zone_map(self, zone: Any, visited: set) -> None:
        """ASCII-style zone map showing visited/unvisited nodes."""
        locations = getattr(zone, "locations", {})
        zone_name = getattr(zone, "name", "Unknown Zone")

        lines: List[str] = []
        for loc_id, loc in locations.items():
            loc_name = getattr(loc, "name", loc_id)
            if loc_id in visited:
                marker = f"[bold cyan][ {loc_name} ][/bold cyan]"
            else:
                marker = f"[dim]( {loc_name} )[/dim]"
            lines.append(f"  {marker}")

        map_text = "\n".join(lines) if lines else "[dim]No map data available.[/dim]"

        console.print()
        console.print(
            Panel(
                map_text,
                title=f"[bold magenta]MAP: {zone_name}[/bold magenta]",
                border_style="magenta",
                box=box.ROUNDED,
                padding=(1, 2),
            )
        )

    # -------------------------------------------------------------------------
    # 6. Action list
    # -------------------------------------------------------------------------

    def show_actions(self, available_actions: List[str]) -> None:
        """Show numbered action list in a panel."""
        action_text = Text()
        for i, action in enumerate(available_actions, 1):
            action_text.append(f"  [{i}] ", style="bold yellow")
            action_text.append(f"{action}\n", style="white")

        console.print()
        console.print(
            Panel(
                action_text,
                title="[bold yellow]ACTIONS[/bold yellow]",
                border_style="yellow",
                box=box.ROUNDED,
                padding=(0, 1),
            )
        )

    # -------------------------------------------------------------------------
    # 7. Prompt action
    # -------------------------------------------------------------------------

    def prompt_action(self, actions: List[str]) -> str:
        """Show actions and get player choice, return action string."""
        self.show_actions(actions)
        console.print()
        valid = [str(i) for i in range(1, len(actions) + 1)]
        while True:
            raw = Prompt.ask("[bold yellow]>[/bold yellow] Choose action", show_choices=False)
            if raw.strip() in valid:
                return actions[int(raw.strip()) - 1]
            # Allow typing partial action name
            raw_lower = raw.strip().lower()
            matches = [a for a in actions if raw_lower in a.lower()]
            if len(matches) == 1:
                return matches[0]
            console.print(
                f"[red]Invalid choice. Enter a number 1-{len(actions)}.[/red]"
            )

    # -------------------------------------------------------------------------
    # 8. Combat HUD
    # -------------------------------------------------------------------------

    def show_combat_hud(
        self,
        player_hp: int,
        player_max_hp: int,
        player_energy: int,
        player_max_energy: int,
        enemy_name: str,
        enemy_hp: int,
        enemy_max_hp: int,
    ) -> None:
        """Two-column combat display."""
        player_hp_bar = self._hp_bar(player_hp, player_max_hp, width=18)
        player_en_bar = self._energy_bar(player_energy, player_max_energy, width=18)
        enemy_hp_bar = self._hp_bar(enemy_hp, enemy_max_hp, width=18)

        player_col = Text()
        player_col.append("AMEER USMAN\n", style="bold bright_cyan")
        player_col.append("HP  ", style="bold red")
        player_col.append_text(player_hp_bar)
        player_col.append("\nEN  ", style="bold cyan")
        player_col.append_text(player_en_bar)

        enemy_col = Text()
        enemy_col.append(f"{enemy_name.upper()}\n", style="bold red")
        enemy_col.append("HP  ", style="bold red")
        enemy_col.append_text(enemy_hp_bar)

        grid = Table.grid(expand=True)
        grid.add_column(ratio=1)
        grid.add_column(justify="center", ratio=0, min_width=5)
        grid.add_column(ratio=1)
        grid.add_row(
            Panel(player_col, border_style="bright_cyan", box=box.ROUNDED, padding=(0, 1)),
            Align.center(Text(" VS ", style="bold yellow")),
            Panel(enemy_col, border_style="red", box=box.ROUNDED, padding=(0, 1)),
        )

        console.print()
        console.print(
            Panel(
                grid,
                title="[bold red]-- COMBAT --[/bold red]",
                border_style="red",
                box=box.DOUBLE,
            )
        )

    # -------------------------------------------------------------------------
    # 9. Combat log
    # -------------------------------------------------------------------------

    def show_combat_log(self, messages: List[str]) -> None:
        """Show last 5 combat messages with appropriate colors."""
        recent = messages[-5:] if len(messages) > 5 else messages
        log_text = Text()
        for msg in recent:
            lower = msg.lower()
            if any(k in lower for k in ["miss", "evade", "dodge"]):
                style = "dim yellow"
            elif any(k in lower for k in ["damage", "hit", "strike", "attack"]):
                style = "bold red"
            elif any(k in lower for k in ["heal", "restore", "recover"]):
                style = "green"
            elif any(k in lower for k in ["skill", "ability", "energy"]):
                style = "cyan"
            elif any(k in lower for k in ["defeat", "dead", "destroyed"]):
                style = "bold magenta"
            else:
                style = "white"
            log_text.append(f"  > {msg}\n", style=style)

        console.print(
            Panel(
                log_text,
                title="[bold yellow]COMBAT LOG[/bold yellow]",
                border_style="yellow",
                box=box.ROUNDED,
                padding=(0, 1),
            )
        )

    # -------------------------------------------------------------------------
    # 10. Combat actions
    # -------------------------------------------------------------------------

    def show_combat_actions(self) -> str:
        """Show: (A)ttack, (S)kill, (I)tem, (R)un. Return choice."""
        action_text = Text()
        action_text.append("  [A] ", style="bold red")
        action_text.append("Attack    ", style="white")
        action_text.append("[S] ", style="bold cyan")
        action_text.append("Skill    ", style="white")
        action_text.append("[I] ", style="bold yellow")
        action_text.append("Item    ", style="white")
        action_text.append("[R] ", style="bold dim")
        action_text.append("Run\n", style="white")

        console.print(
            Panel(
                action_text,
                border_style="red",
                box=box.ROUNDED,
                padding=(0, 1),
            )
        )
        console.print()
        while True:
            choice = Prompt.ask(
                "[bold red]>[/bold red] Combat action",
                show_choices=False,
            ).lower().strip()
            if choice in ("a", "attack"):
                return "attack"
            if choice in ("s", "skill"):
                return "skill"
            if choice in ("i", "item"):
                return "item"
            if choice in ("r", "run"):
                return "run"
            console.print("[red]Enter A, S, I, or R.[/red]")

    # -------------------------------------------------------------------------
    # 11. Skill menu
    # -------------------------------------------------------------------------

    def show_skill_menu(
        self, skills: List[str], skill_defs: dict, player_energy: int
    ) -> str:
        """Show available skills, return chosen skill_id or 'cancel'."""
        table = Table(
            title="SKILLS",
            box=box.ROUNDED,
            border_style="cyan",
            show_header=True,
            header_style="bold cyan",
        )
        table.add_column("#", style="yellow", width=3)
        table.add_column("Skill", style="bold white")
        table.add_column("Cost", style="cyan", justify="right")
        table.add_column("Description", style="dim white")

        usable_indices = []
        for i, skill_id in enumerate(skills, 1):
            skill = skill_defs.get(skill_id, {})
            name = skill.get("name", skill_id)
            cost = skill.get("energy_cost", 0)
            desc = skill.get("description", "")
            affordable = player_energy >= cost
            row_style = "white" if affordable else "dim"
            cost_str = f"{cost} EN"
            if not affordable:
                cost_str += " [red](no energy)[/red]"
                name = f"[dim]{name}[/dim]"
            else:
                usable_indices.append(str(i))
            table.add_row(str(i), name, cost_str, desc, style=row_style)

        table.add_row("0", "[dim]Cancel[/dim]", "", "")

        console.print()
        console.print(table)
        console.print(f"\n  Current Energy: [cyan]{player_energy}[/cyan]\n")

        while True:
            choice = Prompt.ask(
                "[cyan]>[/cyan] Choose skill (0 to cancel)", show_choices=False
            ).strip()
            if choice == "0":
                return "cancel"
            if choice in usable_indices:
                return skills[int(choice) - 1]
            console.print(
                "[red]Invalid or unaffordable choice. Enter a valid number.[/red]"
            )

    # -------------------------------------------------------------------------
    # 12. Inventory
    # -------------------------------------------------------------------------

    def show_inventory(self, items: List[dict], context: str = "explore") -> str:
        """Show inventory, return chosen item_id or 'cancel'."""
        if not items:
            console.print(
                Panel(
                    "[dim]Your inventory is empty.[/dim]",
                    title="[bold yellow]INVENTORY[/bold yellow]",
                    border_style="yellow",
                    box=box.ROUNDED,
                )
            )
            input("  Press Enter to continue...")
            return "cancel"

        action_label = "Use" if context == "explore" else "Use in combat"

        table = Table(
            title="INVENTORY",
            box=box.ROUNDED,
            border_style="yellow",
            show_header=True,
            header_style="bold yellow",
        )
        table.add_column("#", style="yellow", width=3)
        table.add_column("Icon", width=4)
        table.add_column("Item", style="bold white")
        table.add_column("Qty", style="cyan", justify="right", width=5)
        table.add_column("Description", style="dim white")

        for i, item in enumerate(items, 1):
            icon = item.get("icon", "?")
            name = item.get("name", item.get("id", "Unknown"))
            qty = str(item.get("quantity", 1))
            desc = item.get("description", "")
            table.add_row(str(i), icon, name, qty, desc)

        table.add_row("0", "", "[dim]Cancel[/dim]", "", "")

        console.print()
        console.print(table)
        console.print()

        while True:
            choice = Prompt.ask(
                f"[yellow]>[/yellow] {action_label} which item? (0 to cancel)",
                show_choices=False,
            ).strip()
            if choice == "0":
                return "cancel"
            if choice.isdigit() and 1 <= int(choice) <= len(items):
                return items[int(choice) - 1].get("id", items[int(choice) - 1].get("name", "unknown"))
            console.print(f"[red]Enter a number 0-{len(items)}.[/red]")

    # -------------------------------------------------------------------------
    # 13. Dialogue
    # -------------------------------------------------------------------------

    def show_dialogue(
        self, npc_name: str, message: str, options: List[str]
    ) -> int:
        """Show NPC dialogue panel, return chosen option index (0-based)."""
        speech = Text()
        speech.append(f'"{message}"', style="italic white")

        option_text = Text("\n")
        for i, opt in enumerate(options, 1):
            option_text.append(f"  [{i}] ", style="bold cyan")
            option_text.append(f"{opt}\n", style="white")

        body = Text.assemble(speech, "\n", option_text)

        console.print()
        console.print(
            Panel(
                body,
                title=f"[bold yellow]{npc_name.upper()}[/bold yellow]",
                border_style="yellow",
                box=box.ROUNDED,
                padding=(1, 2),
            )
        )
        console.print()

        valid = [str(i) for i in range(1, len(options) + 1)]
        while True:
            choice = Prompt.ask(
                "[cyan]>[/cyan] Your response", show_choices=False
            ).strip()
            if choice in valid:
                return int(choice) - 1
            console.print(f"[red]Enter a number 1-{len(options)}.[/red]")

    # -------------------------------------------------------------------------
    # 14. Narrative reveal
    # -------------------------------------------------------------------------

    def show_narrative(self, text: str, title: str = "") -> None:
        """Show story text in elegant panel with slow reveal."""
        console.print()
        displayed = ""
        panel_title = f"[bold magenta]{title}[/bold magenta]" if title else "[bold magenta]-- TRANSMISSION --[/bold magenta]"

        # Build character-by-character reveal inside a Live context
        with Live(
            Panel(
                Text("", style="italic white"),
                title=panel_title,
                border_style="magenta",
                box=box.DOUBLE,
                padding=(1, 2),
            ),
            console=console,
            refresh_per_second=30,
        ) as live:
            for char in text:
                displayed += char
                live.update(
                    Panel(
                        Text(displayed, style="italic white"),
                        title=panel_title,
                        border_style="magenta",
                        box=box.DOUBLE,
                        padding=(1, 2),
                    )
                )
                time.sleep(0.018)

        console.print()
        input("  [ Press Enter to continue ] ")

    # -------------------------------------------------------------------------
    # 15. Lore panel
    # -------------------------------------------------------------------------

    def show_lore(self, title: str, text: str) -> None:
        """Show discovered lore in a special styled panel."""
        lore_body = Text()
        lore_body.append(f"{text}", style="italic dim white")

        console.print()
        console.print(
            Panel(
                lore_body,
                title=f"[bold bright_cyan]DATA FRAGMENT: {title.upper()}[/bold bright_cyan]",
                subtitle="[dim cyan][ RECOVERED FROM NEXUS ARCHIVE ][/dim cyan]",
                border_style="bright_cyan",
                box=box.DOUBLE,
                padding=(1, 2),
            )
        )
        console.print()
        input("  [ Press Enter ] ")

    # -------------------------------------------------------------------------
    # 16. Level up
    # -------------------------------------------------------------------------

    def show_level_up(self, level: int, new_skill: Optional[str] = None) -> None:
        """Celebration panel for level up."""
        body = Text(justify="center")
        body.append(f"\n  LEVEL {level}  \n\n", style="bold bright_yellow")
        body.append("All systems upgraded. Neural pathways expanding.\n", style="white")
        if new_skill:
            body.append(f"\n  NEW SKILL UNLOCKED: ", style="bold cyan")
            body.append(f"{new_skill}\n", style="bold bright_cyan")

        console.print()
        console.print(
            Panel(
                Align.center(body),
                title="[bold bright_yellow]-- LEVEL UP! --[/bold bright_yellow]",
                border_style="bright_yellow",
                box=box.DOUBLE,
                padding=(1, 4),
            )
        )
        console.print()
        input("  [ Press Enter ] ")

    # -------------------------------------------------------------------------
    # 17. Zone complete
    # -------------------------------------------------------------------------

    def show_zone_complete(self, zone_name: str, nexus_lore: str) -> None:
        """Zone completion screen."""
        body = Text(justify="center")
        body.append(f"\n  ZONE SECURED: {zone_name.upper()}  \n\n", style="bold green")
        body.append("You have pushed NEXUS back. For now.\n\n", style="white")
        body.append("NEXUS TRANSMISSION:\n", style="bold magenta")
        body.append(f'  "{nexus_lore}"\n', style="italic magenta")

        console.print()
        console.print(
            Panel(
                Align.center(body),
                title="[bold green]-- ZONE COMPLETE --[/bold green]",
                border_style="green",
                box=box.DOUBLE,
                padding=(1, 4),
            )
        )
        console.print()
        input("  [ Press Enter to continue ] ")

    # -------------------------------------------------------------------------
    # 18. Victory
    # -------------------------------------------------------------------------

    def show_victory(self, zones_cleared: int) -> None:
        """Victory screen."""
        self._clear()
        art = (
            "  ██╗   ██╗██╗ ██████╗████████╗ ██████╗ ██████╗ ██╗   ██╗  \n"
            "  ██║   ██║██║██╔════╝╚══██╔══╝██╔═══██╗██╔══██╗╚██╗ ██╔╝  \n"
            "  ██║   ██║██║██║        ██║   ██║   ██║██████╔╝ ╚████╔╝   \n"
            "  ╚██╗ ██╔╝██║██║        ██║   ██║   ██║██╔══██╗  ╚██╔╝    \n"
            "   ╚████╔╝ ██║╚██████╗   ██║   ╚██████╔╝██║  ██║   ██║     \n"
            "    ╚═══╝  ╚═╝ ╚═════╝   ╚═╝    ╚═════╝ ╚═╝  ╚═╝   ╚═╝     \n"
        )
        body = Text(justify="center")
        body.append(art, style="bold bright_cyan")
        body.append(f"\n  NEXUS has been contained.\n", style="bold white")
        body.append(f"  Zones cleared: {zones_cleared}\n", style="yellow")
        body.append(
            "\n  Humanity lives. The Future Vision endures.\n",
            style="italic white",
        )
        body.append(
            "\n  Ameer Usman — the man who built NEXUS,\n"
            "  and the man who stopped it.\n",
            style="italic dim cyan",
        )

        console.print()
        console.print(
            Panel(
                Align.center(body),
                title="[bold bright_cyan]-- SAFETYNET: COMPLETE --[/bold bright_cyan]",
                border_style="bright_cyan",
                box=box.DOUBLE,
                padding=(2, 4),
            )
        )
        console.print()
        input("  [ Press Enter to exit ] ")

    # -------------------------------------------------------------------------
    # 19. Game over
    # -------------------------------------------------------------------------

    def show_game_over(self) -> None:
        """Game over screen."""
        self._clear()
        body = Text(justify="center")
        body.append(
            "\n  ██████╗  █████╗ ███╗   ███╗███████╗     \n"
            "  ██╔════╝ ██╔══██╗████╗ ████║██╔════╝     \n"
            "  ██║  ███╗███████║██╔████╔██║█████╗       \n"
            "  ██║   ██║██╔══██║██║╚██╔╝██║██╔══╝       \n"
            "  ╚██████╔╝██║  ██║██║ ╚═╝ ██║███████╗     \n"
            "   ╚═════╝ ╚═╝  ╚═╝╚═╝     ╚═╝╚══════╝     \n"
            "\n  ██████╗ ██╗   ██╗███████╗██████╗        \n"
            "  ██╔═══██╗██║   ██║██╔════╝██╔══██╗       \n"
            "  ██║   ██║██║   ██║█████╗  ██████╔╝       \n"
            "  ██║   ██║╚██╗ ██╔╝██╔══╝  ██╔══██╗       \n"
            "  ╚██████╔╝ ╚████╔╝ ███████╗██║  ██║       \n"
            "   ╚═════╝   ╚═══╝  ╚══════╝╚═╝  ╚═╝       \n",
            style="bold red",
        )
        body.append("\n  NEXUS wins. The world goes dark.\n", style="italic dim red")
        body.append(
            '  "You built me to be perfect, Ameer. Did you expect anything less?"\n',
            style="italic magenta",
        )
        body.append("\n  — NEXUS\n", style="dim magenta")

        console.print()
        console.print(
            Panel(
                Align.center(body),
                title="[bold red]-- SYSTEM FAILURE --[/bold red]",
                border_style="red",
                box=box.DOUBLE,
                padding=(1, 4),
            )
        )
        console.print()
        input("  [ Press Enter ] ")

    # -------------------------------------------------------------------------
    # 20. Save indicator
    # -------------------------------------------------------------------------

    def show_save_indicator(self) -> None:
        """Brief 'Game Saved' notification."""
        console.print(
            Panel(
                Text("  Game Saved  ", style="bold green", justify="center"),
                border_style="green",
                box=box.ROUNDED,
                padding=(0, 2),
            )
        )
        time.sleep(0.8)

    # -------------------------------------------------------------------------
    # 21. Loading spinner
    # -------------------------------------------------------------------------

    def show_loading(self, message: str = "Processing...", duration: float = 1.5) -> None:
        """Spinner for async-feeling operations."""
        with Live(
            Panel(
                Align.center(
                    Text.assemble(
                        Spinner("dots", style="bold cyan"),
                        " ",
                        Text(message, style="bold cyan"),
                    )
                ),
                border_style="dim cyan",
                box=box.ROUNDED,
                padding=(0, 2),
            ),
            console=console,
            refresh_per_second=12,
        ):
            time.sleep(duration)

    # -------------------------------------------------------------------------
    # 22. Item found
    # -------------------------------------------------------------------------

    def show_item_found(self, item_name: str, item_desc: str, icon: str) -> None:
        """Item discovery popup."""
        body = Text(justify="center")
        body.append(f"\n  {icon}  ITEM ACQUIRED  {icon}\n\n", style="bold bright_yellow")
        body.append(f"  {item_name}\n", style="bold white")
        body.append(f"  {item_desc}\n", style="italic dim white")

        console.print()
        console.print(
            Panel(
                Align.center(body),
                title="[bold bright_yellow]-- FOUND --[/bold bright_yellow]",
                border_style="bright_yellow",
                box=box.ROUNDED,
                padding=(1, 4),
            )
        )
        console.print()
        input("  [ Press Enter ] ")

    # -------------------------------------------------------------------------
    # 23. Enemy encounter
    # -------------------------------------------------------------------------

    def show_enemy_encounter(
        self, enemy_name: str, enemy_desc: str, dialogue: str
    ) -> None:
        """Enemy encounter screen."""
        body = Text()
        body.append(f"  {enemy_desc}\n\n", style="white")
        body.append("  HOSTILE TRANSMISSION:\n", style="bold red")
        body.append(f'  "{dialogue}"\n', style="italic red")

        console.print()
        console.print(
            Panel(
                body,
                title=f"[bold red]-- ENCOUNTER: {enemy_name.upper()} --[/bold red]",
                border_style="red",
                box=box.DOUBLE,
                padding=(1, 2),
            )
        )
        console.print()
        input("  [ Press Enter to engage ] ")

    # -------------------------------------------------------------------------
    # 24. Stats panel
    # -------------------------------------------------------------------------

    def show_stats_panel(self, state: GameState) -> None:
        """Full stats panel: all stats, skills, story progress."""
        # Core stats table
        stats_table = Table(
            box=box.ROUNDED, border_style="cyan", show_header=False, padding=(0, 1)
        )
        stats_table.add_column("Stat", style="bold cyan", width=18)
        stats_table.add_column("Value", style="white")

        stats_table.add_row("Level", str(state.level))
        stats_table.add_row("Experience", f"{state.xp} XP")
        stats_table.add_row(
            "HP",
            f"{state.hp}/{state.max_hp}  " + _bar_str(state.hp, state.max_hp, 12),
        )
        stats_table.add_row(
            "Energy",
            f"{state.energy}/{state.max_energy}  " + _bar_str(state.energy, state.max_energy, 12, "cyan"),
        )
        stats_table.add_row("Attack", str(getattr(state, "attack", "N/A")))
        stats_table.add_row("Defense", str(getattr(state, "defense", "N/A")))
        stats_table.add_row("Zone", state.current_zone)
        stats_table.add_row("Location", state.current_location)

        # Skills table
        skills_table = Table(
            title="Skills",
            box=box.ROUNDED,
            border_style="bright_cyan",
            header_style="bold bright_cyan",
        )
        skills_table.add_column("Skill ID", style="cyan")
        skills_table.add_column("Unlocked", style="green")

        skills = getattr(state, "skills", []) or []
        if skills:
            for sk in skills:
                skills_table.add_row(sk, "[green]YES[/green]")
        else:
            skills_table.add_row("[dim]none[/dim]", "")

        # Flags
        flags_table = Table(
            title="Story Flags",
            box=box.ROUNDED,
            border_style="magenta",
            header_style="bold magenta",
        )
        flags_table.add_column("Flag", style="magenta")

        flags = getattr(state, "story_flags", set()) or set()
        if flags:
            for flag in sorted(flags):
                flags_table.add_row(flag)
        else:
            flags_table.add_row("[dim]none[/dim]")

        console.print()
        console.print(
            Panel(
                stats_table,
                title="[bold cyan]AMEER USMAN — STATS[/bold cyan]",
                border_style="cyan",
                box=box.DOUBLE,
                padding=(1, 2),
            )
        )
        console.print(skills_table)
        console.print(flags_table)
        console.print()
        input("  [ Press Enter ] ")

    # -------------------------------------------------------------------------
    # 25. Narrate (typing effect)
    # -------------------------------------------------------------------------

    def narrate(self, text: str) -> None:
        """Simple narrative print with typing effect (char by char, ~0.02s delay)."""
        console.print()
        for char in text:
            console.print(char, end="", style="italic white")
            time.sleep(0.02)
        console.print()
        console.print()


# ---------------------------------------------------------------------------
# Module-level helper (used inside show_stats_panel without self)
# ---------------------------------------------------------------------------

def _bar_str(current: int, maximum: int, width: int = 12, colour: str = "green") -> str:
    """Return a plain string bar (for embedding in table cells)."""
    if maximum <= 0:
        maximum = 1
    ratio = max(0, min(1, current / maximum))
    filled = int(ratio * width)
    empty = width - filled
    return "█" * filled + "░" * empty
