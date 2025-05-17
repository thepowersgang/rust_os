# 
# rust_os SystemTest - Test framework/wrapper
# 
import QemuMonitor
import re
import time
import os
import shutil
import sys
import subprocess

def run_test(arch, test_name,  test_method):
    instance = Instance(arch, test_name)
    try:
        test_method( instance )
    except TestFail as e:
        print("--- FAILED")
        instance.flush()
        print("TEST FAILURE:",e)
        sys.exit(1)

class TestFail(Exception):
    def __init__(self, reason: str):
        self.reason = reason
    def __repr__(self):
        return "TestFail(%r)" % (self.reason,)

def test_assert(reason: str, condition):
    if condition == False:
        raise TestFail(reason)
    print("STEP:", reason)

RE_threadSwitch = re.compile(r'\d+[td] \d+/TID\d+\[kernel::threads\] - (.*)')
assert RE_threadSwitch.search('\x1b[0000m  9844t 0/TID10[kernel::threads] - L270: reschedule() - No active threads, idling\x1b[0m'), RE_threadSwitch.pattern
assert RE_threadSwitch.search('\x1b[0000m  9849d 0/TID1[kernel::threads] - Idle task switch to 0xffff800000000480(2 IRQ Worker)\x1b[0m')
assert RE_threadSwitch.search("\x1b[0000m 22380d 0/TID2[kernel::threads] - Task switch to 0xffff800000000bc0(4 GUI Timer)\x1b[0m")
assert RE_threadSwitch.search('\x1b[0000m 18802d 0/TID1[kernel::threads] - Idle task switch to 0xffff800000000480(2 IRQ Worker)\x1b[0m')

class Instance:
    def __init__(self, arch: str, testname: str):
        self._cmd = QemuMonitor.QemuMonitor(["make", "-C", "rundir/", "ARCH={}".format(arch,), "NOTEE=1"])
        self.lastlog: "list[str]" = []
        self._testname = testname
        self._screenshot_idx: int = 0
        self._x: int = 0
        self._y: int = 0
        self._btns: int = 0
        self._screenshot_dir = 'test-{}-{}'.format(arch,testname,)
        self._cmd.cmd("change vnc :99")
        try:
            shutil.rmtree("rundir/"+self._screenshot_dir)
        except:
            pass
        os.mkdir("rundir/"+self._screenshot_dir)
        pass
    def start_capture(self):
        self._encoder = subprocess.Popen([
            '/home/tpg/.local/bin/flvrec.py',
            '-o', 'rundir/'+self._screenshot_dir+'/video.flv',
            'localhost:99'
            ])
    def flush(self):
        try:
            while self.wait_for_idle():
                pass
        except TestFail as e:
            print( "{!r}".format(e,) )
        
    def __del__(self):
        self._cmd.send_screendump('{}/z-final.ppm'.format(self._screenshot_dir))

    
    _counter_wait_for_line = 0
    def _check_for_panic(self, line: str):
        if re.search(r'\d+k \d+/TID\d+\[kernel::unwind\] - ', line) != None:
            raise TestFail("Kernel panic")
        if re.search(r'\d+d \d+/TID\d+\[syscalls\] - USER> PANIC: ', line) != None:
            raise TestFail("User panic")
    def wait_for_line(self, regex: "re.Pattern[str]|str", timeout: float):
        self._counter_wait_for_line += 1
        print("wait_for_line[{}]({!r}, timeout={:.1f})".format(self._counter_wait_for_line, regex, timeout))
        self.lastlog = []
        end_time = time.time() + timeout
        while True:
            line = self._cmd.get_line(timeout=end_time - time.time())
            if line == None:
                return False
            if line != "":
                print("wait_for_line[{}]: {!r}".format(self._counter_wait_for_line, line))
                self._check_for_panic(line)
                rv = re.search(regex, line)
                if rv != None:
                    return rv
                self.lastlog.append( line )
            if time.time() > end_time:
                return False
    
    def wait_for_idle(self, timeout: float = 10.0, idle_time: float = 2):
        self._counter_wait_for_line += 1
        print("wait_for_idle[{}](timeout={:.1f},idle_time={:.1f})".format(self._counter_wait_for_line, timeout, idle_time))
        fail_after = time.time() + timeout
        pass_after = float('inf')   # time.time() + idle_time
        maybe_idle = False
        # TODO: Ensure that it's idle for at least `n` seconds?
        while True:
            line = self._cmd.get_line(timeout=min(pass_after, fail_after) - time.time())
            if time.time() >= fail_after:
                return False
            if time.time() >= pass_after:
                return True
            if line is None:
                print("wait_for_idle: This shouldn't happen, `get_line` timeout but timeouts not hit")
                return False
            if line != "":
                print("wait_for_idle[{}]: {!r}".format(self._counter_wait_for_line, line))
                self.lastlog.append( line )
            self._check_for_panic(line)
            # Look for a thread switch log line
            m = RE_threadSwitch.search(line)
            if m is not None:
                to_idle = False
                # Ignore specific matched log lines
                # - A switch to idling
                if " - No active threads, idling" in m[1] is not None:
                    to_idle = True
                # - A switch to the GUI timer thread or the IRQ worker
                #   - This does assume that the GUI timer doesn't do too much
                if "ask switch to 0x" in m[1]:
                    if " IRQ Worker)" in m[1]:
                        to_idle = True
                    if " GUI Timer)" in m[1]:
                        to_idle = True
                print("wait_for_idle: {}->{}: pass_after={}".format(maybe_idle, to_idle, pass_after))
                # If switching to an idle task, then start the idle timer (if not started)
                if to_idle:
                    if not maybe_idle:
                        maybe_idle = True
                        pass_after = time.time() + idle_time
                    else:
                        pass
                # Reset the idle timer if there was a thread switch to anything else
                else:
                    maybe_idle = False
                    pass_after = float('inf')
            else:
                # Ignore any other message
                pass
        pass
    

    def match_line(self, name: str, pattern: str, matches: "list[str]", timeout=5):
        """
        Wait for a line that matches the provided pattern, and assert that it fits the provided matches
        """
        line = self.wait_for_line(pattern, timeout=timeout)
        test_assert("%s - Match timeout: %s" % (name, pattern,), line != False)
        assert line != False
        for i,m in enumerate(matches):
            if line.group(i+1) != m:
                raise TestFail("%s - Unexpected match from \"%s\" - %i: %r != %r" % (name, pattern, i, line.group(1+i), m,))
    
    
    def wait_startapp(self, path: str, timeout=5):
        """
        TIFFLIN - Wait for the userland entrypoint to be invoked, and check the binary name
        """
        line = self.wait_for_line(r"\[syscalls\] - USER> Calling entry 0x[0-9a-f]+ for b\"(.*)\"", timeout=timeout)
        test_assert("Start timeout: %s" % (path,), line != False)
        assert line != False
        if line.group(1) != path:
            raise TestFail("Unexpected binary start: %r != %r" % (line.group(1), path,))
    
    def type_string(self, string: str):
        """
        Type a string using the VM's keyboard
        """
        for c in string:
            if 'a' <= c <= 'z':
                self._cmd.send_key(c)
            elif 'A' <= c <= 'Z':
                self._cmd.send_combo(['shift', c.lower()])
            elif c == '\n':
                self._cmd.send_key('ret')
            elif c == ' ':
                self._cmd.send_key('spc')
            elif c == '/':
                self._cmd.send_key('slash')
            else:
                print( "ERROR: Unknown character '%s' in type_string".format(c) )
                raise RuntimeError("Test error: Unknown character")
    def type_key(self, key: str):
        self._cmd.send_key(key)
    def type_combo(self, keys: "list[str]"):
        self._cmd.send_combo(keys)
    def mouse_to(self, x: int, y: int):
        dx, dy = x - self._x, y - self._y
        self._cmd.mouse_move(dx,dy)
        self._x = x
        self._y = y
    def mouse_press(self, btn):
        assert btn >= 1
        assert btn <= 3
        self._btns |= 1 << (btn-1)
        self._cmd.mouse_button(self._btns)
    def mouse_release(self, btn):
        assert btn >= 1
        assert btn <= 3
        self._btns &= ~(1 << (btn-1))
        self._cmd.mouse_button(self._btns)

    def screenshot(self, tag):
        self._cmd.send_screendump('%s/%s-%s.ppm' % (self._screenshot_dir, self._screenshot_idx, tag))
        self._screenshot_idx += 1

