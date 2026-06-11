#!/usr/bin/env python3
"""
SAFETYNET: FUTURE VISION
========================
A multi-agent terminal RPG | Personalized for Ameer Usman

Run: python -m game.main
     python run_game.py

Optionally set ANTHROPIC_API_KEY in .env for live AI-powered gameplay.
"""
import os
import sys

# Load .env if present
try:
    from dotenv import load_dotenv
    load_dotenv()
except ImportError:
    pass


def check_dependencies() -> bool:
    missing = []
    try:
        import rich  # noqa
    except ImportError:
        missing.append("rich")
    try:
        import pydantic  # noqa
    except ImportError:
        missing.append("pydantic")

    if missing:
        print(f"Missing dependencies: {', '.join(missing)}")
        print("Run: pip install -r requirements.txt")
        return False
    return True


def main() -> None:
    if not check_dependencies():
        sys.exit(1)

    # Check for API key and inform user
    api_key = os.getenv("ANTHROPIC_API_KEY", "")
    if api_key and api_key != "sk-ant-your-key-here":
        pass  # UI will show AI-powered mode
    else:
        # API key not set — run in offline mode (rich pre-written content)
        pass

    from game.engine import GameEngine
    engine = GameEngine()
    try:
        engine.run()
    except KeyboardInterrupt:
        print("\n\n[Signal received] SafetyNet interface shutdown. Stay safe, Ameer.\n")
        sys.exit(0)


if __name__ == "__main__":
    main()
