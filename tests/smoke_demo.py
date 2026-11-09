"""Optional real-browser Rule-90 regression. Requires Playwright and a browser."""
import functools
import http.server
from pathlib import Path
import sys
import threading
from playwright.sync_api import sync_playwright


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


def main():
    demo = Path(__file__).resolve().parents[1] / 'demo'
    server = http.server.ThreadingHTTPServer(
        ('127.0.0.1', 0), functools.partial(QuietHandler, directory=str(demo)))
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    origin = f'http://127.0.0.1:{server.server_port}'
    errors = []
    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(
                channel='msedge' if sys.platform == 'win32' else None, headless=True)
            try:
                page = browser.new_page()
                page.route('**/*', lambda route: route.continue_()
                           if route.request.url.startswith(origin + '/') else route.abort())
                page.on('pageerror', lambda error: errors.append(str(error)))
                page.goto(origin + '/', wait_until='networkidle')
                assert page.get_by_role('note').is_visible()
                page.locator('#ruleset-select').select_option('90')
                for steps, a, b in [(16, 105, 42), (11, 3, 240), (37, 128, 127)]:
                    page.locator('#key-gen-steps').fill(str(steps))
                    page.locator('#key-gen-steps').dispatch_event('input')
                    page.locator('#input-a').fill(str(a))
                    page.locator('#input-b').fill(str(b))
                    page.locator('#run-calc-btn').click()
                    assert page.locator('#calc-result-val').inner_text() == str(a ^ b)
                    assert page.locator('#sandbox-badge').inner_text() == 'EXAMPLE MATCH'
                assert not errors, errors
            finally:
                browser.close()
    finally:
        server.shutdown()
        server.server_close()
    print('3 Rule-90 browser XOR cases passed; research notice visible; no uncaught page errors.')


if __name__ == '__main__':
    main()
