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
        print "--- FAILED"
        instance.flush()
        print "TEST FAILURE:",e
        sys.exit(1)

class TestFail:
    def __init__(self, reason):
        self.reason = reason
    def __repr__(self):
        return "TestFail(%r)" % (self.reason,)

def test_assert(reason, condition):
    if condition == False:
        raise TestFail(reason)
    print "STEP:",reason

class Instance:
    def __init__(self, arch, testname):
        self._cmd = QemuMonitor.QemuMonitor(["make", "-C", "Kernel/rundir/", "ARCH=%s" % (arch,), "NOTEE=1"])
        self.lastlog = []
        self._testname = testname
        self._screenshot_idx = 0
        self._x = 0
        self._y = 0
        self._btns = 0
        self._screenshot_dir = 'test-%s-%s' % (arch,testname,)
        self._cmd.cmd("change vnc :99")
        try:
            shutil.rmtree("Kernel/rundir/"+self._screenshot_dir)
        except:
            pass
        os.mkdir("Kernel/rundir/"+self._screenshot_dir)
        pass
    def start_capture(self):
        self._encoder = subprocess.Popen(['/home/tpg/.local/bin/flvrec.py', '-o', 'Kernel/rundir/'+self._screenshot_dir+'/video.flv', 'localhost:99'])
    def flush(self):
        try:
            while self.wait_for_idle():
                pass
        except TestFail as e:
            print "%r" % (e,)
        
    def __del__(self):
        self._cmd.send_screendump('%s/z-final.ppm' % (self._screenshot_dir,))

    
    def wait_for_line(self, regex, timeout):
        self.lastlog = []
        end_time = time.time() + timeout
        while True:
            line = self._cmd.get_line(timeout=end_time - time.time())
            if line == None:
                return False
            if line != "":
                print "wait_for_line - ",line
                if re.search('\d+k \d+\[kernel::unwind\] - ', line) != None:
                    raise TestFail("Kernel panic")
                if re.search('\d+d \d+\[syscalls\] - USER> PANIC: ', line) != None:
                    raise TestFail("User panic")
                rv = re.search(regex, line)
                if rv != None:
                    return rv
                self.lastlog.append( line )
            if time.time() > end_time:
                return False
    
    def wait_for_idle(self, timeout=1.0, idle_time=0.5):
        end_time = time.time() + timeout
        # TODO: Ensure that it's idle for at least `n` seconds?
        while True:
            if time.time() > end_time:
                return False
            if not self.wait_for_line('\d+t \d+\[kernel::threads\] - L\d+: reschedule\(\) - No active threads, idling', end_time - time.time()):
                return False
            if False == self.wait_for_line('', idle_time):
                return True
    

    def match_line(self, name, pattern, matches, timeout=5):
        """
        Wait for a line that matches the provided pattern, and assert that it fits the provided matches
        """
        line = self.wait_for_line(pattern, timeout=timeout)
        test_assert("%s - Match timeout: %s" % (name, pattern,), line)
        for i,m in enumerate(matches):
            if line.group(i+1) != m:
                raise TestFail("%s - Unexpected match from \"%s\" - %i: %r != %r" % (name, pattern, i, line.group(1+i), m,))
    
    
    def wait_startapp(self, path, timeout=5):
        """
        TIFFLIN - Wait for the userland entrypoint to be invoked, and check the binary name
        """
        line = self.wait_for_line("\[syscalls\] - USER> Calling entry 0x[0-9a-f]+ for b\"(.*)\"", timeout=timeout)
        test_assert("Start timeout: %s" % (path,), line)
        if line.group(1) != path:
            raise TestFail("Unexpected binary start: %r != %r" % (line.group(1), path,))
    
    def type_string(self, string):
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
                print "ERROR: Unknown character '%s' in type_string" % (c)
                raise "Doop"
    def type_key(self, key):
        self._cmd.send_key(key)
    def type_combo(self, keys):
        self._cmd.send_combo(keys)
    def mouse_to(self, x,y):
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

