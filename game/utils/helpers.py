"""
helpers.py
==========
Utility helpers for SAFETYNET: FUTURE VISION.

Cross-cutting helpers for the terminal RPG: dramatic text rendering, screen
management, dice/probability math, and Rich-powered animations. Everything is
console-safe and degrades gracefully if a TTY isn't available.
"""

import time
import random
import os
import sys

from rich.console import Console
from rich.progress import Progress, SpinnerColumn, TextColumn

console = Console()


def slow_print(text: str, delay: float = 0.03) -> None:
    """Print text character by character for dramatic effect.

    Args:
        text: The string to render.
        delay: Seconds to wait between characters. Set to 0 for instant.
    """
    for char in text:
        sys.stdout.write(char)
        sys.stdout.flush()
        if delay > 0 and char != "\n":
            time.sleep(delay)
    sys.stdout.write("\n")
    sys.stdout.flush()


def clear_screen() -> None:
    """Clear the terminal screen (cross-platform)."""
    os.system("cls" if os.name == "nt" else "clear")


def dramatic_pause(seconds: float = 1.0) -> None:
    """Sleep for a beat, with a subtle spinner so the UI never looks frozen.

    Args:
        seconds: How long to pause.
    """
    if seconds <= 0:
        return
    try:
        with console.status("[dim cyan]…[/dim cyan]", spinner="dots"):
            time.sleep(seconds)
    except Exception:
        # Fallback for non-interactive terminals.
        time.sleep(seconds)


def loading_animation(message: str, duration: float = 2.0) -> None:
    """Show a Rich loading spinner with a label for ``duration`` seconds.

    Args:
        message: Text shown next to the spinner.
        duration: How long to animate before returning.
    """
    try:
        with Progress(
            SpinnerColumn(spinner_name="aesthetic", style="bold green"),
            TextColumn("[bold green]{task.description}[/bold green]"),
            transient=True,
            console=console,
        ) as progress:
            task = progress.add_task(message, total=None)
            end = time.time() + duration
            while time.time() < end:
                progress.advance(task, 1)
                time.sleep(0.08)
    except Exception:
        console.print(f"[green]{message}…[/green]")
        time.sleep(duration)


def format_stat_bar(
    current: int,
    maximum: int,
    width: int = 20,
    fill_char: str = "█",
    empty_char: str = "░",
) -> str:
    """Return a colored Rich stat bar string.

    The bar is color-coded by percentage: green when healthy, yellow when
    wounded, red when critical.

    Args:
        current: Current value.
        maximum: Maximum value (clamped to >= 1 to avoid divide-by-zero).
        width: Total character width of the bar.
        fill_char: Character used for filled segments.
        empty_char: Character used for empty segments.

    Returns:
        A Rich-markup string like ``[green]████████░░[/green] 80/100``.
    """
    maximum = max(1, maximum)
    current = clamp(current, 0, maximum)
    ratio = current / maximum
    filled = int(round(ratio * width))
    filled = clamp(filled, 0, width)
    empty = width - filled

    if ratio > 0.5:
        color = "green"
    elif ratio > 0.25:
        color = "yellow"
    else:
        color = "red"

    bar = fill_char * filled + empty_char * empty
    return f"[{color}]{bar}[/{color}] {current}/{maximum}"


def roll_dice(sides: int = 20, modifier: int = 0) -> int:
    """Roll an ``n``-sided die and apply a flat modifier.

    Args:
        sides: Number of faces on the die (minimum 1).
        modifier: Flat bonus/penalty added to the roll.

    Returns:
        The roll total (die face + modifier).
    """
    sides = max(1, sides)
    return random.randint(1, sides) + modifier


def chance(percent: int) -> bool:
    """Return True with the given percentage probability.

    Args:
        percent: Probability 0-100. Values are clamped to that range.

    Returns:
        True roughly ``percent``% of the time.
    """
    percent = clamp(percent, 0, 100)
    if percent <= 0:
        return False
    if percent >= 100:
        return True
    return random.randint(1, 100) <= percent


def clamp(value: int, min_val: int, max_val: int) -> int:
    """Clamp ``value`` to the inclusive range [min_val, max_val]."""
    if min_val > max_val:
        min_val, max_val = max_val, min_val
    return max(min_val, min(max_val, value))


def pad_center(text: str, width: int = 60) -> str:
    """Center ``text`` within ``width`` columns using spaces.

    If the text is wider than ``width`` it is returned unchanged.

    Args:
        text: The text to center.
        width: Target column width.

    Returns:
        The space-padded, centered string.
    """
    if len(text) >= width:
        return text
    return text.center(width)
