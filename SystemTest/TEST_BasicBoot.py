import sys
import TestInstance
from TestInstance import test_assert

def test(instance: TestInstance.Instance):
    test_assert("Kernel image start timed out", instance.wait_for_line("OK43e6H", timeout=10) != False)
    instance.match_line(
        "Init load timed out",
        r"Entering userland at 0x[0-9a-f]+ '([^']+)' '([^']+)'",
        ['/sysroot/bin/loader','/sysroot/bin/init'],
        timeout=10
        )
    instance.match_line(
        "Init start",
        r"\[syscalls\] - USER> Calling entry 0x[0-9a-f]+ for INIT b\"(.*)\"",
        ['/sysroot/bin/init'],
        timeout=5
        )
    instance.wait_startapp("/sysroot/bin/daemon_network", timeout=5)
    # - Check that login spawned
    instance.wait_startapp("/sysroot/bin/login", timeout=10)

    test_assert("Initial startup idle", instance.wait_for_idle(timeout=20))
    instance.screenshot('Login')

try:
    test( TestInstance.Instance("amd64", "Basic") )
except TestInstance.TestFail as e:
    print("TEST FAILURE:",e)
    sys.exit(1)
