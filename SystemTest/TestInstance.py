import QemuMonitor
import re
import time

class TestFail:
    def __init__(self, reason):
        self.reason = reason
    def __repr__(self):
        return "TestFail(%r)" % (self.reason,)

def test_assert(reason, condition):
    if not condition:
        raise TestFail(reason)
    print "STEP:",reason

class Instance:
    def __init__(self, arch, testname):
        self._cmd = QemuMonitor.QemuMonitor(["make", "-C", "Kernel/rundir/", "ARCH=%s" % (arch,), "NOTEE=1"])
        self.lastlog = []
        self._testname = testname
        self._screenshot_idx = 0
        pass
    
    def wait_for_line(self, regex, timeout):
        self.lastlog = []
        now = time.time()
        while True:
            line = self._cmd.get_line(timeout=timeout)
            if line == None:
                return False
            if line != "":
                print "wait_for_idle - line=",line
                if re.search(regex, line) != None:
                    return True
                self.lastlog.append( line )
            if time.time() - now > timeout:
                return False
    
    def wait_for_idle(self, timeout=1.0):
        return self.wait_for_line('\d+t \d+\[kernel::threads\] - L\d+: reschedule\(\) - No active threads, idling', timeout)
    
    def type_string(self, string):
        for c in string:
            if 'a' <= c <= 'z':
                self._cmd.send_key(c)
            elif 'A' <= c <= 'Z':
                self._cmd.send_combo(['shift', c])
            elif c == '\n':
                self._cmd.send_key('ret')
            elif c == ' ':
                self._cmd.send_key('spc')
            else:
                print "ERROR: Unknown character '%s' in type_string" % (c)
                raise "Doop"
    def type_key(self, key):
        self._cmd.send_key(key)

    def screenshot(self, tag):
        self._cmd.send_screendump('test-%s-%s-%s.ppm' % (self._testname, self._screenshot_idx, tag))
        self._screenshot_idx += 1

