import TestInstance
import sys

from TestInstance import test_assert

def test(instance):
    test_assert("Kernel image start timed out", instance.wait_for_line("OK43e6H", timeout=10))
    test_assert("Init load timed out", instance.wait_for_line("Entering userland at 0x[0-9a-f]+ '/system/Tifflin/bin/loader' '/system/Tifflin/bin/init'", timeout=5))

    test_assert("Initial startup timed out", instance.wait_for_idle(timeout=20))
    instance.screenshot('Login')

    instance.type_string('root')
    while instance.wait_for_idle():
        pass
    instance.type_key('ret')
    test_assert("Username return press timeout", instance.wait_for_idle()) # Press
    test_assert("Username return release timeout", instance.wait_for_idle())
    print instance.lastlog
    # TODO: Have an item in the log here
    
    instance.type_string('password')
    while instance.wait_for_idle():
        pass
    instance.type_key('ret')
    test_assert("Password return press timeout", instance.wait_for_idle()) # Press
    test_assert("Shell startup timeout", instance.wait_for_idle(timeout=5))
    print instance.lastlog
    # TODO: Have an item in the log here

    instance.screenshot('Shell')
    instance.type_key('meta_l')
    assert instance.wait_for_idle() # Press
    assert instance.wait_for_idle(timeout=5);
    #print instance.lastlog
    instance.screenshot('Menu')

    #instance.type_key('down')
    #assert instance.wait_for_idle() # Press
    #assert instance.wait_for_idle(timeout=5);
    ##print instance.lastlog
    
    instance.type_key('ret')
    assert instance.wait_for_idle() # Press
    assert instance.wait_for_idle(timeout=10);
    #print instance.lastlog
    instance.screenshot('CLI')


try:
    test( TestInstance.Instance("amd64", "CLI") )
except TestInstance.TestFail as e:
    print "TEST FAILURE:",e
    sys.exit(1)
