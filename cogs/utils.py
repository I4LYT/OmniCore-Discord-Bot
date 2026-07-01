"""General utility commands for OmniCore."""

from __future__ import annotations

from datetime import datetime, timezone
from typing import Optional

import discord
from discord import app_commands
from discord.ext import commands


class Utils(commands.Cog):
    def __init__(self, bot: commands.Bot) -> None:
        self.bot = bot

    async def public_reply(self, interaction: discord.Interaction, content: Optional[str] = None,
                           embed: Optional[discord.Embed] = None) -> None:
        """Post visible output as a normal bot message, not an APP-style visible response."""
        channel = interaction.channel
        if channel is not None and hasattr(channel, "send"):
            await channel.send(content=content, embed=embed)
            if interaction.response.is_done():
                await interaction.followup.send("✅ Posted publicly.", ephemeral=True)
            else:
                await interaction.response.send_message("✅ Posted publicly.", ephemeral=True)
        else:
            if interaction.response.is_done():
                await interaction.followup.send(content=content, embed=embed)
            else:
                await interaction.response.send_message(content=content, embed=embed)

    @app_commands.command(name="ping", description="Check if OmniCore is online")
    async def ping(self, interaction: discord.Interaction) -> None:
        latency = round(self.bot.latency * 1000)
        await self.public_reply(interaction, f"🏓 Pong! Latency: `{latency}ms`")

    @app_commands.command(name="userinfo", description="Show information about a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to inspect. Leave blank for yourself.")
    async def userinfo(self, interaction: discord.Interaction, user: Optional[discord.Member] = None) -> None:
        guild = interaction.guild
        assert guild is not None
        member = user or interaction.user
        assert isinstance(member, discord.Member)

        embed = discord.Embed(
            title=f"User Info: {member}",
            color=member.color if member.color.value else discord.Color.blurple(),
            timestamp=datetime.now(timezone.utc),
        )
        embed.set_thumbnail(url=member.display_avatar.url)
        embed.add_field(name="User ID", value=str(member.id), inline=True)
        embed.add_field(name="Account Created", value=member.created_at.strftime("%Y-%m-%d %H:%M UTC"), inline=True)
        embed.add_field(name="Joined Server", value=member.joined_at.strftime("%Y-%m-%d %H:%M UTC") if member.joined_at else "Unknown", inline=True)
        embed.add_field(name="Top Role", value=member.top_role.mention, inline=True)
        roles = [role.mention for role in member.roles if role != guild.default_role]
        embed.add_field(name="Roles", value=", ".join(roles) if roles else "None", inline=False)
        await self.public_reply(interaction, embed=embed)

    @app_commands.command(name="serverinfo", description="Show information about this server")
    @app_commands.guild_only()
    async def serverinfo(self, interaction: discord.Interaction) -> None:
        guild = interaction.guild
        assert guild is not None
        embed = discord.Embed(
            title=f"Server Info: {guild.name}",
            color=discord.Color.green(),
            timestamp=datetime.now(timezone.utc),
        )
        if guild.icon:
            embed.set_thumbnail(url=guild.icon.url)
        embed.add_field(name="Server ID", value=str(guild.id), inline=True)
        embed.add_field(name="Owner", value=guild.owner.mention if guild.owner else f"ID: {guild.owner_id}", inline=True)
        embed.add_field(name="Members", value=str(guild.member_count), inline=True)
        embed.add_field(name="Channels", value=str(len(guild.channels)), inline=True)
        embed.add_field(name="Roles", value=str(len(guild.roles)), inline=True)
        embed.add_field(name="Created", value=guild.created_at.strftime("%Y-%m-%d %H:%M UTC"), inline=True)
        await self.public_reply(interaction, embed=embed)

    @app_commands.command(name="avatar", description="Show a user's avatar")
    @app_commands.guild_only()
    @app_commands.describe(user="Member whose avatar to show. Leave blank for yourself.")
    async def avatar(self, interaction: discord.Interaction, user: Optional[discord.Member] = None) -> None:
        member = user or interaction.user
        assert isinstance(member, discord.Member)
        embed = discord.Embed(title=f"Avatar: {member}", color=discord.Color.blurple())
        embed.set_image(url=member.display_avatar.url)
        await self.public_reply(interaction, embed=embed)

    @app_commands.command(name="help", description="Show OmniCore command help")
    async def help_command(self, interaction: discord.Interaction) -> None:
        embed = discord.Embed(
            title="OmniCore Help",
            description=(
                "OmniCore uses slash commands. Visible results are posted as normal bot messages.\n\n"
                "For role commands, make sure the **OmniCore bot role is above** the role it is adding/removing."
            ),
            color=discord.Color.blurple(),
            timestamp=datetime.now(timezone.utc),
        )
        commands_text = [
            "`/mute`, `/unmute`, `/timeout`, `/untimeout`",
            "`/ban`, `/unban`, `/kick`",
            "`/warn`, `/warnings`, `/clearwarnings`",
            "`/purge`, `/lock`, `/unlock`, `/slowmode`",
            "`/nick`, `/roleadd`, `/roleremove`",
            "`/userinfo`, `/serverinfo`, `/avatar`, `/ping`, `/help`",
        ]
        embed.add_field(name="Commands", value="\n".join(commands_text), inline=False)
        embed.add_field(
            name="Role command fix",
            value="Server owner and Administrator now pass permission checks. If roleadd still fails, move the bot role higher.",
            inline=False,
        )
        await self.public_reply(interaction, embed=embed)


async def setup(bot: commands.Bot) -> None:
    await bot.add_cog(Utils(bot))
