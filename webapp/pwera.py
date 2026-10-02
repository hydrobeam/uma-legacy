import asyncio, os, time, json
from playwright.async_api import async_playwright
URL='file://' + os.getcwd() + '/dist/index.html'
async def run_and_read(pg):
    t=time.time(); await pg.click('#run'); await pg.wait_for_function("!document.getElementById('run').disabled", timeout=600000)
    txt=await pg.inner_text('#results'); return round(time.time()-t,1), txt
async def main():
    async with async_playwright() as p:
        b = await p.chromium.launch(); pg = await b.new_page(viewport={'width': 1280, 'height': 1600})
        logs=[]; pg.on('pageerror', lambda e: logs.append(str(e))); pg.on('console', lambda m: m.type=='error' and logs.append(m.text))
        await pg.goto(URL); await pg.wait_for_timeout(800)
        print('GLOBAL hint:', await pg.inner_text('#era-hint'), '|', await pg.inner_text('#r-count'))
        await pg.screenshot(path='shot_era_global.png')
        # switch to GM; set no-guarantee targets and SS-UE rank first
        tg=[{"group":20159,"guaranteed":0},{"group":20049,"guaranteed":0},{"group":20166,"guaranteed":0},{"group":20046,"guaranteed":0},{"group":20161,"guaranteed":0},{"group":21010,"guaranteed":0},{"group":20165,"guaranteed":0},{"group":20203,"guaranteed":0}]
        await pg.evaluate(f"localStorage.setItem('llp:targets', JSON.stringify({json.dumps(tg)})); localStorage.setItem('llp:rank', JSON.stringify('0.20,0.70,0.10')); localStorage.setItem('llp:deck', JSON.stringify('opt'));")
        await pg.click('#era-gm'); await pg.wait_for_load_state('load'); await pg.wait_for_timeout(1200)
        print('GM hint:', (await pg.inner_text('#era-hint'))[:90], '|', await pg.inner_text('#r-count'), '|', await pg.inner_text('#results'))
        sec, txt = await run_and_read(pg); print('GM run', sec, 's\n', txt[:900])
        await pg.screenshot(path='shot_era_gm.png', full_page=False)
        await pg.select_option('#x-rand', '1'); sec, txt = await run_and_read(pg); print('GM rand=1', sec, txt.split('\n')[2:5])
        await pg.check('#x-dates'); sec, txt = await run_and_read(pg); print('GM rand=1 dates', sec, txt.split('\n')[2:5])
        # back to global, check own settings kept
        await pg.click('#era-global'); await pg.wait_for_load_state('load'); await pg.wait_for_timeout(800)
        print('back GLOBAL:', await pg.inner_text('#r-count'))
        pg2 = await b.new_page(viewport={'width': 390, 'height': 1600}); await pg2.goto(URL); await pg2.wait_for_timeout(600)
        print('phone scrollWidth', await pg2.evaluate('document.documentElement.scrollWidth'))
        print('errors', logs); await b.close()
asyncio.run(main())
