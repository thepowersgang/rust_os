import TestInstance
from TestInstance import test_assert

def test(instance):
    test_assert("Kernel image start timed out", instance.wait_for_line("OK43e6H", timeout=10))
    test_assert("Init load timed out", instance.wait_for_line("Entering userland at 0x[0-9a-f]+ '/system/Tifflin/bin/loader' '/system/Tifflin/bin/init'", timeout=5))

    test_assert("Initial startup idle", instance.wait_for_idle(timeout=20))
    instance.screenshot('Login')

try:
    test( TestInstance.Instance("amd64", "Basic") )
except TestInstance.TestFail as e:
    print "TEST FAILURE:",e
    sys.exit(1)
