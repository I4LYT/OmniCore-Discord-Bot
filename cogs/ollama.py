"""Ollama mention replies for OmniCore."""

from __future__ import annotations

import asyncio
import os
from typing import Any

import aiohttp
import discord
from discord.ext import commands


DEFAULT_BASE_URL = "http://127.0.0.1:11434"
DEFAULT_SYSTEM_PROMPT = (
    "You are OmniCore, a helpful Discord bot. Answer the user directly, "
    "keep replies concise unless detail is requested, and avoid pinging users."
)
DISCORD_MESSAGE_LIMIT = 1900


class OllamaError(RuntimeError):
    """Raised when Ollama cannot produce a usable reply."""


class OllamaChat(commands.Cog):
    def __init__(self, bot: commands.Bot) -> None:
        self.bot = bot
        self.base_url = os.getenv("OLLAMA_BASE_URL", DEFAULT_BASE_URL).rstrip("/")
        self.model = os.getenv("OLLAMA_MODEL", "").strip()
        self.system_prompt = os.getenv("OLLAMA_SYSTEM_PROMPT", DEFAULT_SYSTEM_PROMPT).strip()
        self.timeout = aiohttp.ClientTimeout(total=self._env_float("OLLAMA_TIMEOUT_SECONDS", 120.0))
        self.session: aiohttp.ClientSession | None = None

    async def cog_load(self) -> None:
        self.session = aiohttp.ClientSession(timeout=self.timeout)

    async def cog_unload(self) -> None:
        if self.session is not None:
            await self.session.close()
            self.session = None

    @commands.Cog.listener()
    async def on_message(self, message: discord.Message) -> None:
        if message.author.bot or self.bot.user is None:
            return
        if self.bot.user not in message.mentions:
            return

        prompt = self._prompt_from_message(message)
        if not prompt:
            await message.reply(
                "Mention me with a question or prompt and I will ask Ollama.",
                mention_author=False,
                allowed_mentions=discord.AllowedMentions.none(),
            )
            return

        async with message.channel.typing():
            try:
                response = await self._ask_ollama(message, prompt)
            except OllamaError as exc:
                response = f"I could not get a response from Ollama: {exc}"

        await self._send_discord_reply(message, response)

    def _prompt_from_message(self, message: discord.Message) -> str:
        content = message.content or ""
        if self.bot.user is not None:
            content = content.replace(f"<@{self.bot.user.id}>", "")
            content = content.replace(f"<@!{self.bot.user.id}>", "")
        return " ".join(content.split()).strip()

    async def _ask_ollama(self, message: discord.Message, prompt: str) -> str:
        session = self._session()
        model = await self._model_name(session)
        channel_name = getattr(message.channel, "name", "direct-message")
        user_prompt = (
            f"Discord user {message.author.display_name} asked in #{channel_name}:\n"
            f"{prompt}\n\n"
            "Reply as the bot in this Discord channel."
        )
        payload: dict[str, Any] = {
            "model": model,
            "prompt": user_prompt,
            "system": self.system_prompt,
            "stream": False,
        }

        try:
            async with session.post(f"{self.base_url}/api/generate", json=payload) as response:
                if response.status >= 400:
                    raise OllamaError(await self._error_text(response))
                data = await response.json(content_type=None)
        except asyncio.TimeoutError as exc:
            raise OllamaError("the request timed out") from exc
        except aiohttp.ClientConnectorError as exc:
            raise OllamaError(f"could not connect to {self.base_url}") from exc
        except aiohttp.ClientError as exc:
            raise OllamaError(str(exc)) from exc

        answer = str(data.get("response", "")).strip()
        if not answer:
            raise OllamaError("Ollama returned an empty response")
        return answer

    async def _model_name(self, session: aiohttp.ClientSession) -> str:
        if self.model:
            return self.model

        try:
            async with session.get(f"{self.base_url}/api/tags") as response:
                if response.status >= 400:
                    raise OllamaError(await self._error_text(response))
                data = await response.json(content_type=None)
        except asyncio.TimeoutError as exc:
            raise OllamaError("the model lookup timed out") from exc
        except aiohttp.ClientConnectorError as exc:
            raise OllamaError(f"could not connect to {self.base_url}") from exc
        except aiohttp.ClientError as exc:
            raise OllamaError(str(exc)) from exc

        models = data.get("models") or []
        for model in models:
            name = model.get("model") or model.get("name")
            if name:
                self.model = str(name)
                return self.model

        raise OllamaError("no Ollama models are installed; run `ollama pull llama3.2` or set OLLAMA_MODEL")

    async def _error_text(self, response: aiohttp.ClientResponse) -> str:
        try:
            data = await response.json(content_type=None)
        except (aiohttp.ClientError, ValueError):
            data = None

        if isinstance(data, dict) and data.get("error"):
            return str(data["error"])

        text = (await response.text()).strip()
        return text or f"HTTP {response.status}"

    def _session(self) -> aiohttp.ClientSession:
        if self.session is None:
            raise OllamaError("HTTP session is not ready")
        return self.session

    def _env_float(self, name: str, default: float) -> float:
        value = os.getenv(name)
        if value is None:
            return default
        try:
            return float(value)
        except ValueError:
            return default

    async def _send_discord_reply(self, message: discord.Message, response: str) -> None:
        chunks = self._split_for_discord(response)
        allowed_mentions = discord.AllowedMentions.none()

        for index, chunk in enumerate(chunks):
            if index == 0:
                await message.reply(chunk, mention_author=False, allowed_mentions=allowed_mentions)
            else:
                await message.channel.send(chunk, allowed_mentions=allowed_mentions)

    def _split_for_discord(self, text: str) -> list[str]:
        remaining = text.strip() or "Ollama returned an empty response."
        chunks: list[str] = []

        while len(remaining) > DISCORD_MESSAGE_LIMIT:
            split_at = remaining.rfind("\n", 0, DISCORD_MESSAGE_LIMIT)
            if split_at < DISCORD_MESSAGE_LIMIT // 2:
                split_at = remaining.rfind(" ", 0, DISCORD_MESSAGE_LIMIT)
            if split_at < DISCORD_MESSAGE_LIMIT // 2:
                split_at = DISCORD_MESSAGE_LIMIT

            chunks.append(remaining[:split_at].strip())
            remaining = remaining[split_at:].strip()

        chunks.append(remaining)
        return chunks


async def setup(bot: commands.Bot) -> None:
    await bot.add_cog(OllamaChat(bot))
