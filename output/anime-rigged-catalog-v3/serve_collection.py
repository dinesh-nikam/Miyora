"""Serve only this generated collection over loopback for local model review."""
import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import sys
import threading
import webbrowser

PREVIEW_REQUEST_BACKLOG=64


class CollectionHTTPServer(ThreadingHTTPServer):
    request_queue_size=PREVIEW_REQUEST_BACKLOG


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--port',type=int,default=1425)
    parser.add_argument('--open',action='store_true')
    args=parser.parse_args()
    if not 1024<=args.port<=65535:
        parser.error('Choose a port between 1024 and 65535')
    root=Path(__file__).resolve().parent
    try:
        server=CollectionHTTPServer(('127.0.0.1',args.port),partial(SimpleHTTPRequestHandler,directory=str(root)))
    except OSError:
        print('Preview port unavailable. Use --port with another local port.',file=sys.stderr)
        return 1
    url=f'http://127.0.0.1:{args.port}/'
    print(f'Collection viewer: {url} — press Ctrl+C to stop',flush=True)
    if args.open:
        threading.Timer(.5,lambda:webbrowser.open(url)).start()
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()
    return 0


if __name__=='__main__':
    raise SystemExit(main())
