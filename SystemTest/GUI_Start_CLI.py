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
    # TODO: Have an item in the log here
    
    instance.type_string('password')
    # - Wait until there's 1s with no action
    while instance.wait_for_idle():
        pass
    instance.type_key('ret')
    test_assert("Password return press timeout", instance.wait_for_idle()) # Press
    test_assert("Shell startup timeout", instance.wait_for_line("\[syscalls\] - USER> Calling entry 0x[0-9a-f]+ for b\"/sysroot/bin/shell\"", timeout=5))
    test_assert("Shell idle timeout", instance.wait_for_idle(timeout=5))
    # TODO: Have an item in the log here

    # - Open the "System" menu (press left windows key)
    instance.screenshot('Shell')
    instance.type_key('meta_l')
    test_assert("System menu press timeout", instance.wait_for_idle()) # Press
    test_assert("System menu release timeout", instance.wait_for_idle(timeout=5)) # Release
    instance.screenshot('Menu')

    # - Select the top item to open the CLI
    instance.type_key('ret')
    assert instance.wait_for_idle() # Press
    test_assert("CLI startup timeout", instance.wait_for_line("\[syscalls\] - USER> Calling entry 0x[0-9a-f]+ for b\"/sysroot/bin/simple_console\"", timeout=5))
    test_assert("CLI idle timeout", instance.wait_for_idle(timeout=5));
    instance.screenshot('CLI')


try:
    test( TestInstance.Instance("amd64", "CLI") )
except TestInstance.TestFail as e:
    print "TEST FAILURE:",e
    sys.exit(1)
