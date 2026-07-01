"""
OmniCore moderation commands.

Important design notes for this fixed version:
- Visible command output is sent with channel.send(), so it appears as a normal
  bot message in the server channel. The slash interaction itself only receives
  a tiny private acknowledgement so Discord does not mark the command as failed.
- The server owner and users with Administrator always pass permission checks.
- Commands do not use default_member_permissions in the slash-command decorator.
  This prevents Discord's slash-command permission UI from blocking the owner.
"""

from __future__ import annotations

import json
import os
from datetime import datetime, timedelta, timezone
from typing import Optional

import discord
from discord import app_commands
from discord.ext import commands


MOD_LOG_CHANNEL_NAME = "mod-log"


def parse_duration(text: str) -> Optional[timedelta]:
    """Convert durations like 10m, 2h, 1d, or 30 into a timedelta.

    If the user enters only a number, it is treated as minutes.
    """
    text = text.strip().lower()
    if not text:
        return None

    try:
        if text[-1] in {"s", "m", "h", "d"}:
            amount = int(text[:-1])
            unit = text[-1]
        else:
            amount = int(text)
            unit = "m"
    except ValueError:
        return None

    if amount <= 0:
        return None

    if unit == "s":
        return timedelta(seconds=amount)
    if unit == "m":
        return timedelta(minutes=amount)
    if unit == "h":
        return timedelta(hours=amount)
    if unit == "d":
        return timedelta(days=amount)
    return None


class Moderation(commands.Cog):
    def __init__(self, bot: commands.Bot) -> None:
        self.bot = bot
        data_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "data"))
        os.makedirs(data_dir, exist_ok=True)
        self.warning_file = os.path.join(data_dir, "warnings.json")
        self.warnings = self.load_warnings()

    # ------------------------------------------------------------------
    # Helper methods
    # ------------------------------------------------------------------

    def load_warnings(self) -> dict:
        if not os.path.exists(self.warning_file):
            return {}
        try:
            with open(self.warning_file, "r", encoding="utf-8") as f:
                return json.load(f)
        except json.JSONDecodeError:
            return {}

    def save_warnings(self) -> None:
        with open(self.warning_file, "w", encoding="utf-8") as f:
            json.dump(self.warnings, f, indent=2)

    def is_owner_or_admin(self, interaction: discord.Interaction) -> bool:
        """Return True if the command user is the server owner or an admin."""
        guild = interaction.guild
        user = interaction.user
        if guild is None:
            return False
        if guild.owner_id == user.id:
            return True
        if isinstance(user, discord.Member) and user.guild_permissions.administrator:
            return True
        return False

    def has_perm(self, interaction: discord.Interaction, permission_name: str) -> bool:
        """Check a permission while always allowing owner/admin.

        This is the main fix for your issue: even if Discord reports one
        permission weirdly, the server owner and Administrator users pass.
        """
        if self.is_owner_or_admin(interaction):
            return True
        user = interaction.user
        if isinstance(user, discord.Member):
            return bool(getattr(user.guild_permissions, permission_name, False))
        return False

    async def public_reply(self, interaction: discord.Interaction, content: Optional[str] = None,
                           embed: Optional[discord.Embed] = None) -> None:
        """Send the real command output publicly as a normal bot message.

        Slash command responses show Discord's APP label. A normal channel.send()
        message appears as the bot account's regular server message. We still
        privately acknowledge the slash command so Discord does not show
        "interaction failed".
        """
        channel = interaction.channel
        if channel is not None and hasattr(channel, "send"):
            await channel.send(content=content, embed=embed)
        else:
            # Fallback if Discord gives us a weird channel object.
            if interaction.response.is_done():
                await interaction.followup.send(content=content, embed=embed)
            else:
                await interaction.response.send_message(content=content, embed=embed)
            return

        # Private acknowledgement only. The visible output was already sent above.
        try:
            if interaction.response.is_done():
                await interaction.followup.send("✅ Posted publicly.", ephemeral=True)
            else:
                await interaction.response.send_message("✅ Posted publicly.", ephemeral=True)
        except discord.InteractionResponded:
            pass

    async def log_mod_action(self, guild: discord.Guild, embed: discord.Embed) -> None:
        channel = discord.utils.get(guild.text_channels, name=MOD_LOG_CHANNEL_NAME)
        if channel:
            try:
                await channel.send(embed=embed)
            except discord.Forbidden:
                pass

    def basic_embed(self, title: str, description: str, color: discord.Color) -> discord.Embed:
        return discord.Embed(title=title, description=description, color=color, timestamp=datetime.now(timezone.utc))

    def bot_can_manage_role(self, guild: discord.Guild, role: discord.Role) -> bool:
        me = guild.me
        if me is None:
            return False
        if not me.guild_permissions.manage_roles and not me.guild_permissions.administrator:
            return False
        # Discord role hierarchy rule: bots can only manage roles lower than their top role.
        return role < me.top_role

    # ------------------------------------------------------------------
    # Moderation commands
    # ------------------------------------------------------------------

    @app_commands.command(name="mute", description="Mute a member with the Muted role")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to mute", reason="Reason for the mute")
    async def mute(self, interaction: discord.Interaction, user: discord.Member,
                   reason: str = "No reason provided") -> None:
        if not self.has_perm(interaction, "manage_roles"):
            await self.public_reply(interaction, "❌ You need Manage Roles or Administrator to use `/mute`.")
            return

        guild = interaction.guild
        assert guild is not None
        muted_role = discord.utils.get(guild.roles, name="Muted")

        if muted_role is None:
            try:
                muted_role = await guild.create_role(name="Muted", reason="OmniCore mute command setup")
                for channel in guild.channels:
                    try:
                        await channel.set_permissions(muted_role, send_messages=False, speak=False, add_reactions=False)
                    except (discord.Forbidden, discord.HTTPException):
                        continue
            except discord.Forbidden:
                await self.public_reply(interaction, "❌ I could not create a Muted role. Move my bot role higher and give me Manage Roles.")
                return

        if muted_role in user.roles:
            await self.public_reply(interaction, f"ℹ️ {user.mention} is already muted.")
            return

        if not self.bot_can_manage_role(guild, muted_role):
            await self.public_reply(interaction, "❌ I cannot manage the Muted role. Move my bot role above the Muted role.")
            return

        try:
            await user.add_roles(muted_role, reason=reason)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot mute that member. My highest role may be too low.")
            return

        msg = f"🔇 {user.mention} has been muted. Reason: {reason}"
        await self.public_reply(interaction, msg)
        embed = self.basic_embed("Member Muted", msg, discord.Color.orange())
        embed.add_field(name="Moderator", value=interaction.user.mention)
        await self.log_mod_action(guild, embed)

    @app_commands.command(name="unmute", description="Remove the Muted role from a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to unmute")
    async def unmute(self, interaction: discord.Interaction, user: discord.Member) -> None:
        if not self.has_perm(interaction, "manage_roles"):
            await self.public_reply(interaction, "❌ You need Manage Roles or Administrator to use `/unmute`.")
            return

        guild = interaction.guild
        assert guild is not None
        muted_role = discord.utils.get(guild.roles, name="Muted")
        if muted_role is None or muted_role not in user.roles:
            await self.public_reply(interaction, f"ℹ️ {user.mention} is not muted.")
            return

        try:
            await user.remove_roles(muted_role, reason="Unmuted by OmniCore")
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot remove that role. My highest role may be too low.")
            return

        msg = f"🔊 {user.mention} has been unmuted."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Member Unmuted", msg, discord.Color.green()))

    @app_commands.command(name="timeout", description="Timeout a member for a duration like 10m, 2h, or 1d")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to timeout", duration="Example: 10m, 2h, 1d", reason="Reason for timeout")
    async def timeout(self, interaction: discord.Interaction, user: discord.Member,
                      duration: str, reason: str = "No reason provided") -> None:
        if not self.has_perm(interaction, "moderate_members"):
            await self.public_reply(interaction, "❌ You need Timeout Members/Moderate Members or Administrator to use `/timeout`.")
            return

        delta = parse_duration(duration)
        if delta is None:
            await self.public_reply(interaction, "❌ Invalid duration. Try `10m`, `2h`, or `1d`.")
            return
        if delta > timedelta(days=28):
            await self.public_reply(interaction, "❌ Discord timeouts cannot be longer than 28 days.")
            return

        try:
            await user.timeout(delta, reason=reason)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot timeout that member. My highest role may be too low.")
            return

        guild = interaction.guild
        assert guild is not None
        msg = f"⏲️ {user.mention} has been timed out for `{duration}`. Reason: {reason}"
        await self.public_reply(interaction, msg)
        embed = self.basic_embed("Member Timed Out", msg, discord.Color.blue())
        embed.add_field(name="Moderator", value=interaction.user.mention)
        await self.log_mod_action(guild, embed)

    @app_commands.command(name="untimeout", description="Remove a member's timeout")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to remove timeout from")
    async def untimeout(self, interaction: discord.Interaction, user: discord.Member) -> None:
        if not self.has_perm(interaction, "moderate_members"):
            await self.public_reply(interaction, "❌ You need Timeout Members/Moderate Members or Administrator to use `/untimeout`.")
            return

        try:
            await user.timeout(None, reason="Timeout removed by OmniCore")
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot remove that timeout. My highest role may be too low.")
            return

        guild = interaction.guild
        assert guild is not None
        msg = f"✅ Timeout removed from {user.mention}."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Timeout Removed", msg, discord.Color.green()))

    @app_commands.command(name="ban", description="Ban a member from the server")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to ban", reason="Reason for ban")
    async def ban(self, interaction: discord.Interaction, user: discord.Member,
                  reason: str = "No reason provided") -> None:
        if not self.has_perm(interaction, "ban_members"):
            await self.public_reply(interaction, "❌ You need Ban Members or Administrator to use `/ban`.")
            return

        try:
            await user.ban(reason=reason)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot ban that member. My highest role may be too low.")
            return

        guild = interaction.guild
        assert guild is not None
        msg = f"🚫 {user.mention} has been banned. Reason: {reason}"
        await self.public_reply(interaction, msg)
        embed = self.basic_embed("Member Banned", msg, discord.Color.red())
        embed.add_field(name="Moderator", value=interaction.user.mention)
        await self.log_mod_action(guild, embed)

    @app_commands.command(name="unban", description="Unban a user by user ID")
    @app_commands.guild_only()
    @app_commands.describe(user_id="Discord user ID to unban")
    async def unban(self, interaction: discord.Interaction, user_id: str) -> None:
        if not self.has_perm(interaction, "ban_members"):
            await self.public_reply(interaction, "❌ You need Ban Members or Administrator to use `/unban`.")
            return

        guild = interaction.guild
        assert guild is not None
        try:
            user = await self.bot.fetch_user(int(user_id))
            await guild.unban(user, reason="Unbanned by OmniCore")
        except ValueError:
            await self.public_reply(interaction, "❌ That is not a valid user ID.")
            return
        except discord.NotFound:
            await self.public_reply(interaction, "ℹ️ That user is not banned.")
            return
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I do not have permission to unban users.")
            return

        msg = f"✅ {user} has been unbanned."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("User Unbanned", msg, discord.Color.green()))

    @app_commands.command(name="kick", description="Kick a member from the server")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to kick", reason="Reason for kick")
    async def kick(self, interaction: discord.Interaction, user: discord.Member,
                   reason: str = "No reason provided") -> None:
        if not self.has_perm(interaction, "kick_members"):
            await self.public_reply(interaction, "❌ You need Kick Members or Administrator to use `/kick`.")
            return

        try:
            await user.kick(reason=reason)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot kick that member. My highest role may be too low.")
            return

        guild = interaction.guild
        assert guild is not None
        msg = f"👢 {user.mention} has been kicked. Reason: {reason}"
        await self.public_reply(interaction, msg)
        embed = self.basic_embed("Member Kicked", msg, discord.Color.orange())
        embed.add_field(name="Moderator", value=interaction.user.mention)
        await self.log_mod_action(guild, embed)

    @app_commands.command(name="warn", description="Warn a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to warn", reason="Reason for warning")
    async def warn(self, interaction: discord.Interaction, user: discord.Member,
                   reason: str = "No reason provided") -> None:
        if not self.has_perm(interaction, "moderate_members"):
            await self.public_reply(interaction, "❌ You need Timeout Members/Moderate Members or Administrator to use `/warn`.")
            return

        guild = interaction.guild
        assert guild is not None
        guild_id = str(guild.id)
        user_id = str(user.id)
        self.warnings.setdefault(guild_id, {}).setdefault(user_id, []).append({
            "reason": reason,
            "moderator_id": str(interaction.user.id),
            "timestamp": datetime.now(timezone.utc).isoformat(),
        })
        self.save_warnings()

        msg = f"⚠️ {user.mention} has been warned. Reason: {reason}"
        await self.public_reply(interaction, msg)
        embed = self.basic_embed("Member Warned", msg, discord.Color.gold())
        embed.add_field(name="Moderator", value=interaction.user.mention)
        await self.log_mod_action(guild, embed)

    @app_commands.command(name="warnings", description="Show warnings for a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member whose warnings you want to view")
    async def warnings_cmd(self, interaction: discord.Interaction, user: discord.Member) -> None:
        if not self.has_perm(interaction, "moderate_members"):
            await self.public_reply(interaction, "❌ You need Timeout Members/Moderate Members or Administrator to use `/warnings`.")
            return

        guild = interaction.guild
        assert guild is not None
        user_warnings = self.warnings.get(str(guild.id), {}).get(str(user.id), [])
        if not user_warnings:
            await self.public_reply(interaction, f"✅ {user.mention} has no warnings.")
            return

        embed = self.basic_embed(f"Warnings for {user}", f"Total warnings: {len(user_warnings)}", discord.Color.gold())
        for index, warning in enumerate(user_warnings, start=1):
            moderator = guild.get_member(int(warning["moderator_id"]))
            mod_text = moderator.mention if moderator else warning["moderator_id"]
            embed.add_field(
                name=f"Warning #{index}",
                value=f"Reason: {warning['reason']}\nModerator: {mod_text}\nTime: {warning['timestamp']}",
                inline=False,
            )
        await self.public_reply(interaction, embed=embed)

    @app_commands.command(name="clearwarnings", description="Clear all warnings for a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member whose warnings to clear")
    async def clearwarnings(self, interaction: discord.Interaction, user: discord.Member) -> None:
        if not self.has_perm(interaction, "moderate_members"):
            await self.public_reply(interaction, "❌ You need Timeout Members/Moderate Members or Administrator to use `/clearwarnings`.")
            return

        guild = interaction.guild
        assert guild is not None
        guild_id = str(guild.id)
        user_id = str(user.id)
        if guild_id in self.warnings and user_id in self.warnings[guild_id]:
            del self.warnings[guild_id][user_id]
            self.save_warnings()
            msg = f"🗑️ Cleared all warnings for {user.mention}."
        else:
            msg = f"ℹ️ {user.mention} had no warnings."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Warnings Updated", msg, discord.Color.green()))

    @app_commands.command(name="purge", description="Delete recent messages in this channel")
    @app_commands.guild_only()
    @app_commands.describe(amount="Number of messages to delete, from 1 to 100")
    async def purge(self, interaction: discord.Interaction, amount: app_commands.Range[int, 1, 100]) -> None:
        if not self.has_perm(interaction, "manage_messages"):
            await self.public_reply(interaction, "❌ You need Manage Messages or Administrator to use `/purge`.")
            return

        channel = interaction.channel
        if not hasattr(channel, "purge"):
            await self.public_reply(interaction, "❌ This command can only be used in text channels.")
            return

        try:
            deleted = await channel.purge(limit=amount)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I do not have permission to delete messages here.")
            return

        guild = interaction.guild
        assert guild is not None
        msg = f"🧹 Deleted {len(deleted)} messages."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Messages Purged", msg, discord.Color.purple()))

    @app_commands.command(name="lock", description="Lock the current channel")
    @app_commands.guild_only()
    async def lock(self, interaction: discord.Interaction) -> None:
        if not self.has_perm(interaction, "manage_channels"):
            await self.public_reply(interaction, "❌ You need Manage Channels or Administrator to use `/lock`.")
            return

        guild = interaction.guild
        channel = interaction.channel
        assert guild is not None
        if not hasattr(channel, "set_permissions"):
            await self.public_reply(interaction, "❌ This command can only be used in server channels.")
            return

        try:
            await channel.set_permissions(guild.default_role, send_messages=False)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot change this channel's permissions.")
            return

        msg = f"🔒 {channel.mention} has been locked."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Channel Locked", msg, discord.Color.dark_red()))

    @app_commands.command(name="unlock", description="Unlock the current channel")
    @app_commands.guild_only()
    async def unlock(self, interaction: discord.Interaction) -> None:
        if not self.has_perm(interaction, "manage_channels"):
            await self.public_reply(interaction, "❌ You need Manage Channels or Administrator to use `/unlock`.")
            return

        guild = interaction.guild
        channel = interaction.channel
        assert guild is not None
        if not hasattr(channel, "set_permissions"):
            await self.public_reply(interaction, "❌ This command can only be used in server channels.")
            return

        try:
            await channel.set_permissions(guild.default_role, send_messages=None)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot change this channel's permissions.")
            return

        msg = f"🔓 {channel.mention} has been unlocked."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Channel Unlocked", msg, discord.Color.green()))

    @app_commands.command(name="slowmode", description="Set slowmode in the current channel")
    @app_commands.guild_only()
    @app_commands.describe(delay="Slowmode delay in seconds, 0 disables it")
    async def slowmode(self, interaction: discord.Interaction, delay: app_commands.Range[int, 0, 21600]) -> None:
        if not self.has_perm(interaction, "manage_channels"):
            await self.public_reply(interaction, "❌ You need Manage Channels or Administrator to use `/slowmode`.")
            return

        channel = interaction.channel
        if not isinstance(channel, discord.TextChannel):
            await self.public_reply(interaction, "❌ Slowmode can only be changed in normal text channels.")
            return

        try:
            await channel.edit(slowmode_delay=delay)
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot edit this channel.")
            return

        msg = f"⏱️ Slowmode set to {delay} seconds in {channel.mention}." if delay else f"⏱️ Slowmode disabled in {channel.mention}."
        await self.public_reply(interaction, msg)

    @app_commands.command(name="nick", description="Change a member's nickname")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to rename", nickname="New nickname")
    async def nick(self, interaction: discord.Interaction, user: discord.Member, nickname: str) -> None:
        if not self.has_perm(interaction, "manage_nicknames"):
            await self.public_reply(interaction, "❌ You need Manage Nicknames or Administrator to use `/nick`.")
            return

        try:
            await user.edit(nick=nickname, reason=f"Nickname changed by {interaction.user}")
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ I cannot change that member's nickname. My highest role may be too low.")
            return

        msg = f"✏️ Changed {user.mention}'s nickname to **{nickname}**."
        await self.public_reply(interaction, msg)

    @app_commands.command(name="roleadd", description="Add a role to a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to give the role to", role="Role to add")
    async def roleadd(self, interaction: discord.Interaction, user: discord.Member, role: discord.Role) -> None:
        # FIXED: owner/admin passes, and command is not hidden by Discord-side default permissions.
        if not self.has_perm(interaction, "manage_roles"):
            await self.public_reply(interaction, "❌ You need Manage Roles or Administrator to use `/roleadd`.")
            return

        guild = interaction.guild
        assert guild is not None
        if role == guild.default_role:
            await self.public_reply(interaction, "❌ You cannot add the @everyone role.")
            return
        if not self.bot_can_manage_role(guild, role):
            await self.public_reply(interaction, "❌ I cannot add that role. Move the OmniCore bot role ABOVE the role you want me to add.")
            return

        try:
            await user.add_roles(role, reason=f"Role added by {interaction.user}")
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ Discord blocked me from adding that role. Check my role position.")
            return

        msg = f"✅ Added {role.mention} to {user.mention}."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Role Added", msg, discord.Color.blurple()))

    @app_commands.command(name="roleremove", description="Remove a role from a member")
    @app_commands.guild_only()
    @app_commands.describe(user="Member to remove the role from", role="Role to remove")
    async def roleremove(self, interaction: discord.Interaction, user: discord.Member, role: discord.Role) -> None:
        # FIXED: owner/admin passes, and command is not hidden by Discord-side default permissions.
        if not self.has_perm(interaction, "manage_roles"):
            await self.public_reply(interaction, "❌ You need Manage Roles or Administrator to use `/roleremove`.")
            return

        guild = interaction.guild
        assert guild is not None
        if role == guild.default_role:
            await self.public_reply(interaction, "❌ You cannot remove the @everyone role.")
            return
        if role not in user.roles:
            await self.public_reply(interaction, f"ℹ️ {user.mention} does not have {role.mention}.")
            return
        if not self.bot_can_manage_role(guild, role):
            await self.public_reply(interaction, "❌ I cannot remove that role. Move the OmniCore bot role ABOVE the role you want me to remove.")
            return

        try:
            await user.remove_roles(role, reason=f"Role removed by {interaction.user}")
        except discord.Forbidden:
            await self.public_reply(interaction, "❌ Discord blocked me from removing that role. Check my role position.")
            return

        msg = f"✅ Removed {role.mention} from {user.mention}."
        await self.public_reply(interaction, msg)
        await self.log_mod_action(guild, self.basic_embed("Role Removed", msg, discord.Color.blurple()))


async def setup(bot: commands.Bot) -> None:
    await bot.add_cog(Moderation(bot))
