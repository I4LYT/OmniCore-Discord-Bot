"""
OmniCore Discord Bot - main entry point.

This version is designed to avoid the issues you were seeing:
1. Slash-command results are posted with channel.send(), so the visible output
   appears as a normal bot/member message in the channel instead of only as an
   ephemeral slash-command response.
2. Permission checks always allow the server owner and anyone with Administrator.
3. Slash commands do not use Discord-side default_member_permissions, so server
   owners are not accidentally blocked by Discord's command permissions UI.
"""

from __future__ import annotations

import asyncio
import logging
import os

import discord
from discord.ext import commands
from dotenv import load_dotenv

load_dotenv()

TOKEN = os.getenv("DISCORD_TOKEN")
if not TOKEN:
    raise RuntimeError("Missing DISCORD_TOKEN. Rename .env.example to .env and paste your bot token there.")

logging.basicConfig(level=logging.INFO)
log = logging.getLogger("omnicore")

# Intents configure what information Discord sends to the bot.
# Server Members Intent must be enabled in the Developer Portal because this bot
# works with members, roles, nicknames, kicks, bans, and timeouts.
# Message Content Intent must be enabled so mentions can be turned into Ollama
# prompts.
intents = discord.Intents.default()
intents.guilds = True
intents.members = True
intents.messages = True
intents.message_content = True
intents.bans = True


class OmniCoreBot(commands.Bot):
    def __init__(self) -> None:
        super().__init__(command_prefix="!", intents=intents)

    async def setup_hook(self) -> None:
        # Load command files.
        await self.load_extension("cogs.moderation")
        await self.load_extension("cogs.utils")
        await self.load_extension("cogs.ollama")

        # Sync slash commands globally. If commands do not update immediately,
        # restart Discord or wait a few minutes.
        synced = await self.tree.sync()
        log.info("Synced %s slash commands globally.", len(synced))

    async def on_ready(self) -> None:
        log.info("Logged in as %s (%s)", self.user, self.user.id if self.user else "unknown")
        await self.change_presence(activity=discord.Game(name="/help | OmniCore"))


async def main() -> None:
    async with OmniCoreBot() as bot:
        await bot.start(TOKEN)


if __name__ == "__main__":
    asyncio.run(main())
