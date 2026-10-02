import asyncio, os, json
from playwright.async_api import async_playwright
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); ctx = await b.new_context(viewport={'width': 1280, 'height': 900}); pg = await ctx.new_page()
        url='file://' + os.getcwd() + '/dist/index.html'
        await pg.goto(url); await pg.wait_for_timeout(300)
        old=[{"group":20159,"guaranteed":0},{"group":20049,"guaranteed":2},{"group":20166,"guaranteed":2},{"group":20046,"guaranteed":2},{"group":20161,"guaranteed":0,"deck_white":0.8},{"group":21010,"guaranteed":0},{"group":20165,"guaranteed":0,"deck_gold":0.8},{"group":20203,"guaranteed":2}]
        await pg.evaluate("t => { localStorage.clear(); localStorage.setItem('llp:targets', JSON.stringify(t)); }", old)
        await pg.reload(); await pg.wait_for_timeout(800)
        for nm in ['Sasami Anshinzawa SSR This Might Sting!', 'Daitaku Helios SSR Sunlit Angel ♪']:
            await pg.get_by_role('button', name=nm).first.click()
        await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=300000)
        t=await pg.inner_text('#results'); i=t.find('#1'); print(t[i:i+220].replace(chr(10),' | ')); await b.close()
asyncio.run(main())
