#!/usr/bin/env python3
"""Servidor de dev do frontend do app: ui/ por cima de ui-dev/, sem cache.

Uso (da raiz do repositório):
    python3 -I scripts/servir-ui-dev.py [porta]      # padrão 5174

Um caminho que não existe em crates/clipsync-app/ui/ é procurado em
crates/clipsync-app/ui-dev/ na mesma URL. Assim o simulador (mock) e a
galeria ficam fora do frontendDist do Tauri, e no navegador tudo continua
nos mesmos caminhos. Cache-Control no-store evita módulo velho no portal.
Escuta só em 127.0.0.1.
"""
import functools
import http.server
import os
import sys

APP = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "clipsync-app")
UI = os.path.realpath(os.path.join(APP, "ui"))
UI_DEV = os.path.realpath(os.path.join(APP, "ui-dev"))


class Sobreposto(http.server.SimpleHTTPRequestHandler):
    def translate_path(self, path):
        primeiro = super().translate_path(path)
        if os.path.exists(primeiro):
            return primeiro
        rel = os.path.relpath(primeiro, self.directory)
        candidato = os.path.realpath(os.path.join(UI_DEV, rel))
        if candidato.startswith(UI_DEV + os.sep) and os.path.exists(candidato):
            return candidato
        return primeiro

    def end_headers(self):
        self.send_header("Cache-Control", "no-store, max-age=0")
        super().end_headers()

    def log_message(self, *args):
        pass


def main():
    porta = int(sys.argv[1]) if len(sys.argv) > 1 else 5174
    servidor = http.server.ThreadingHTTPServer(
        ("127.0.0.1", porta), functools.partial(Sobreposto, directory=UI)
    )
    print(f"servindo {UI} (+ {UI_DEV}) em http://127.0.0.1:{porta}", flush=True)
    servidor.serve_forever()


if __name__ == "__main__":
    main()
