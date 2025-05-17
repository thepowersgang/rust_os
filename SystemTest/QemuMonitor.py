import select
import subprocess
import threading
import time
import typing
from _thread import interrupt_main

class KillerThread:
    def __init__(self):
        self._event = threading.Event()
        self._start = threading.Event()
        self._run = True
        self._th = threading.Thread(target=lambda: self.run())
    def reset(self):
        self._event.set()
    def start(self):
        self._start.set()
    def kill(self):
        self._run = False
        self._start.set()
    def run(self):
        while self._run:
            #print"- Waiting to time")
            self._start.wait()
            if not self._run:
                break
            #print("- Timing 2s")
            self._start.clear()
            if self._event.wait(2.0) == None:
                interrupt_main()
            #print("- Done")
            self._event.clear()

def readline_timeout(stream: "typing.IO[bytes]", timeout=1.0) -> "str|None":
    rv = bytearray()
    end_time = time.time() + timeout
    #print("readline_timeout")
    while end_time > time.time():
        r,_w,_e = select.select( [stream], [], [], end_time - time.time())
        if len(r) > 0:
            v = stream.read(1)
            rv += v
            #print("'%s' %02x" % (v, ord(v)))
            if v == b"\n" or v == b"\r":
                #print "--- --"
                break
        else:
            print("readline_timeout: TIMEOUT ({:.1f}s)".format(timeout))
            break
    if rv == b"":
        return None
    else:
        return rv.decode('utf-8').strip()

class QemuMonitor:
    def __init__(self, cmd_strings: "list[str]"):
        self._instance = subprocess.Popen(cmd_strings, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self._mode = ''
        self._timer = KillerThread()
    def __del__(self):
        self.cmd("quit")
        while True:
            line = self.get_line(timeout=0.5)
            if line == None:
                break
            print("QemuMonitor.__del__ - '{}'".format(line,))
        self._instance.terminate()
        self._timer.kill()
        print("Killing qemu instance")
    def send_key(self, keycode: "str"):
        self.cmd('sendkey %s' % keycode)
    def send_combo(self, keycodes: "list[str]"):
        self.cmd('sendkey %s' % '-'.join(keycodes))
    def mouse_move(self, dx: int, dy: int):
        self.cmd('mouse_move %i %i' % (dx,dy))
    def mouse_button(self, mask: int):
        self.cmd('mouse_button %i' % (mask,))
    
    def get_line(self, timeout=1.0):
        assert self._instance.stdout is not None
        return readline_timeout(self._instance.stdout, timeout)
    
    def send_screendump(self, path):
        self.cmd('screendump {}'.format(path,))

    def cmd(self, string: str):
        assert self._instance.stdin is not None
        if self._mode != 'monitor':
            self._instance.stdin.write(b'\1c')
            self._mode = 'monitor'
            line = self.get_line(timeout=1)
        
        self._instance.stdin.flush()
        self._instance.stdin.write(b'\n')
        print(">> CMD:", string)
        self._instance.stdin.write(string.encode('utf-8'))
        self._instance.stdin.write(b'\n')
        self._instance.stdin.flush()
    
        line = self.get_line(timeout=1)
        print(">> rv =",line)
        line = self.get_line(timeout=1)
        print(">> rv =",line)
        #if line != '(qemu) %s' % (string):
        #    print("Unexpected response: '%s', expected '%s'" % (line, '(qemu) %s' % (string)) )
        #    raise "Doop"
        #line = self.get_line(timeout=1)
        #if line != '(qemu)':
        #    print("Unexpected response: %s" % (line,) )
        #    raise "Doop"
        
