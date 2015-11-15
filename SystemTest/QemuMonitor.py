import select
import subprocess

class QemuMonitor:
    def __init__(self, cmd_strings):
        self._instance = subprocess.Popen(cmd_strings, stdin=subprocess.PIPE, stdout=subprocess.PIPE)
        self._mode = ''
    def __del__(self):
        self.cmd("exit")
        self._instance.terminate()
        print "Killing qemu instance"
    def send_key(self, keycode):
        self.cmd('sendkey %s' % keycode)
    def send_combo(self, keycodes):
        self.cmd('sendkey %s' % '-'.join(keycodes))
    
    def get_line(self, timeout=1.0):
        r,_w,_e = select.select( [self._instance.stdout], [], [], timeout)
        if len(r) > 0:
            s = self._instance.stdout.readline()
            if s == "":
                return None
            return s.strip()
        else:
            return None
    
    def send_screendump(self, path):
        self.cmd('screendump %s' % (path,))

    def cmd(self, string):
        if self._mode != 'monitor':
            self._instance.stdin.write('\1c')
            self._mode = 'monitor'
            line = self.get_line(timeout=1)
        
        self._instance.stdin.flush()
        print ">> CMD:", string
        self._instance.stdin.write(string)
        self._instance.stdin.write('\n')
        self._instance.stdin.flush()
    
        line = self.get_line(timeout=1)
        #if line != '(qemu) %s' % (string):
        #    print "Unexpected response: '%s', expected '%s'" % (line, '(qemu) %s' % (string)) 
        #    raise "Doop"
        #line = self.get_line(timeout=1)
        #if line != '(qemu)':
        #    print "Unexpected response: %s" % (line,) 
        #    raise "Doop"
        
