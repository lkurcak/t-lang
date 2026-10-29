#!/usr/bin/env python3
"""Measure warm hover/hint/reference requests on a generated nine-file program.

Build with `cargo build`, then run `python3 scripts/bench_lsp.py target/debug/t`.
Use identically built binaries when comparing revisions. Each of five batches
sends 60 sequential requests after opening the program and warming all methods.
"""

import json
import pathlib
import statistics
import subprocess
import sys
import tempfile
import time


def measure(binary):
    with tempfile.TemporaryDirectory(prefix='t-lsp-bench-') as directory:
        directory = pathlib.Path(directory)
        modules = [f'module{i}' for i in range(8)]
        main = ''.join(f'mod {name};\n' for name in modules) + 'fn main() {\n'
        for name in modules:
            text = 'fn id<T: Copy>(value: T) -> T { value }\n'
            for index in range(12):
                text += f'pub fn value{index}(value: i64) -> i64 {{ id(value) }}\n'
                main += f'    let _n = {name}::value{index}({index}i64);\n'
            (directory / f'{name}.t').write_text(text)
        main += '}\n'
        root = directory / 'main.t'
        root.write_text(main)
        uri = root.as_uri()
        proc = subprocess.Popen([binary, 'lsp'], stdin=subprocess.PIPE, stdout=subprocess.PIPE)

        def send(message):
            body = json.dumps(dict(jsonrpc='2.0', **message)).encode()
            proc.stdin.write(f'Content-Length: {len(body)}\r\n\r\n'.encode() + body)
            proc.stdin.flush()

        def response(request_id):
            while True:
                length = None
                while True:
                    line = proc.stdout.readline()
                    if not line:
                        raise RuntimeError('server exited')
                    if line == b'\r\n':
                        break
                    if line.lower().startswith(b'content-length:'):
                        length = int(line.split(b':', 1)[1])
                message = json.loads(proc.stdout.read(length))
                if message.get('id') == request_id:
                    assert 'error' not in message, message
                    return message
                if message.get('method') == 'textDocument/publishDiagnostics':
                    assert not [d for d in message['params']['diagnostics'] if d.get('severity') == 1], message

        send(dict(id=1, method='initialize', params={'capabilities': {}}))
        response(1)
        send(dict(method='initialized', params={}))
        send(dict(method='textDocument/didOpen', params={'textDocument': {'uri': uri, 'languageId': 't', 'version': 1, 'text': main}}))
        call_column = main.splitlines()[9].index('::value0') + 3
        methods = [
            ('textDocument/hover', {'position': {'line': 9, 'character': call_column}}),
            ('textDocument/inlayHint', {'range': {'start': {'line': 0, 'character': 0}, 'end': {'line': 106, 'character': 0}}}),
            ('textDocument/references', {'position': {'line': 9, 'character': call_column}, 'context': {'includeDeclaration': True}}),
        ]
        for index, (method, params) in enumerate(methods):
            send(dict(id=10+index, method=method, params={'textDocument': {'uri': uri}, **params}))
            result = response(10+index)['result']
            assert result, (method, result)
        samples = []
        for batch in range(5):
            start = time.perf_counter()
            for index in range(60):
                method, params = methods[index % len(methods)]
                request_id = 100 + batch * 60 + index
                send(dict(id=request_id, method=method, params={'textDocument': {'uri': uri}, **params}))
                response(request_id)
            samples.append(time.perf_counter() - start)
        send(dict(id=9999, method='shutdown'))
        response(9999)
        send(dict(method='exit'))
        assert proc.wait(timeout=10) == 0
        print(json.dumps({'files': 9, 'functions': 105, 'requests_per_batch': 60, 'batches_seconds': samples, 'median_ms_per_request': statistics.median(samples) / 60 * 1000}))


if __name__ == '__main__':
    measure(sys.argv[1] if len(sys.argv) > 1 else 'target/debug/t')
