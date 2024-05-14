#!/usr/bin/env python3

import traceback
import aiohttp
import discord
import asyncio
import os
from os import path
import signal
import functools
import tomllib
import sys

config_file = sys.argv[1] or "spotti-downbot.toml"
config = tomllib.load(open(config_file, mode="rb"))

key = config["key"]
me = config["me"]
sent = False

intents = discord.Intents.default()
intents.members = True
intents.guilds = True
intents.messages = True
intents.message_content = True

bot = discord.Client(intents=intents)


async def aiohttp_get(url):
    async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=15)) as sess:
        async with sess.get(url) as resp:
            await resp.read()
            return resp


@bot.event
async def on_message(m):
    if m.content == "!uptime":
        spotti_text = "spotti did not respond"
        size_text = "size did not respond"

        try:
            spotti = await aiohttp_get(config["spotti_uptime"])
            spotti_text = await spotti.text()
        except:
            pass

        try:
            size = await aiohttp_get(config["size_uptime"])
            size_text = await size.text()
        except:
            pass

        await m.channel.send("spotti: " + spotti_text + "\nsize: " + size_text)


async def send_message(content: str):
    user = await bot.fetch_user(me)
    await user.send("spotti: " + content.strip())


async def check_loop():
    is_broke = False

    while True:
        was_broke = is_broke

        print("tryin")
        first_request = None
        second_request = None

        try:
            first_request = await aiohttp_get(config["spotti_status"])
            first_request.raise_for_status()

            if "global auth was not available" in await first_request.text():
                await send_message("no global auth")
                is_broke = True
            else:
                is_broke = False

        except Exception as e1:
            is_broke = True
            msg = (
                "sad:\n"
                + "".join(traceback.format_exception(e1))
                + (
                    await first_request.text()
                    if first_request is not None
                    else "first request failed\n"
                )
            )

            try:
                second_request = await aiohttp_get(config["spotti_refresh"])
                second_request.raise_for_status()
                print("phew")
                is_broke = False

            except Exception as e2:
                msg += (
                    "refresh also failed:\n"
                    + "".join(traceback.format_exception(e2))
                    + (
                        await second_request.text()
                        if second_request is not None
                        else "second request failed"
                    )
                )
                print("damn")
                await send_message("```\n" + msg + "\n```")

        if was_broke and not is_broke:
            await send_message("thx bby!!!!!!")

        await asyncio.sleep(config["spotti_poll"])


tasks = set()


@bot.event
async def on_ready():
    print(f"{bot.user.name}: {bot.user.id}")
    task = asyncio.create_task(check_loop())
    tasks.add(task)
    task.add_done_callback(tasks.discard)


bot.run(key)
