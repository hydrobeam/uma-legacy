import asyncio, os, time, json
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1000})
        logs = []; pg.on('pageerror', lambda e: logs.append(str(e)))
        await pg.goto('file://' + os.getcwd() + '/dist/index.html'); await pg.wait_for_timeout(800)
        for nm in ['Sasami Anshinzawa SSR This Might Sting!', 'Daitaku Helios SSR Sunlit Angel ♪']:
            await pg.get_by_role('button', name=nm).first.click()
        print(await pg.inner_text('#c-count'))
        # capture job sent
        await pg.evaluate("""() => { const W = window.Worker; }""")
        await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=300000)
        print((await pg.inner_text('#results'))[:500]); print(logs); await b.close()
asyncio.run(main())
