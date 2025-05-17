import TestInstance
import sys
import time

from TestInstance import test_assert,TestFail

KEYNAME_MAP = {
    'meta_l': "LeftGui",
    'ret': "Return",
    'down': "DownArrow",
    'up': "UpArrow",
    
    'alt': "LeftAlt",
    'f4': "F4",
    
    'e': "E",
    't': "T",
    }

def _keypress(instance, key, name, idle=True):
    timeout = 2.0
    if isinstance(key, list):
        instance.type_combo(key)
        for k in key:
            test_assert(name+" "+k+" press timeout", instance.wait_for_line("\[syscalls\] - USER> (Window|Menu)::handle_event\(ev=KeyDown\("+KEYNAME_MAP[k]+"\)\)", timeout=timeout)) # Press
        # - Only assert release for the final key
        k = key[-1]
        test_assert(name+" "+k+" fire timeout", instance.wait_for_line("\[syscalls\] - USER> (Window|Menu)::handle_event\(ev=KeyFire\("+KEYNAME_MAP[k]+"\)\)", timeout=timeout))
    else:
        keyname = KEYNAME_MAP[key]
        instance.type_key(key)
        test_assert(name+" "+key+" press timeout", instance.wait_for_line("\[syscalls\] - USER> (Window|Menu)::handle_event\(ev=KeyDown\("+keyname+"\)\)", timeout=timeout)) # Press
        test_assert(name+" "+key+" fire timeout", instance.wait_for_line("\[syscalls\] - USER> (Window|Menu)::handle_event\(ev=KeyFire\("+keyname+"\)\)", timeout=timeout))
        test_assert(name+" "+key+" release timeout", instance.wait_for_line("\[syscalls\] - USER> (Window|Menu)::handle_event\(ev=KeyUp\("+keyname+"\)\)", timeout=timeout))
        if idle:
            test_assert(name+" "+key+" release idle", instance.wait_for_idle(timeout=5))

def _matchline(instance, name, pattern, matches, timeout=5):
    instance.match_line(name, pattern, matches, timeout)

def _gui_rerender(instance, name, window_names):
    for win in window_names:
        line = instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ '([^']+)' dirty", timeout=10);
        test_assert("%s - Timeout waiting for '%s'" % (name, win,), line)
        if line.group(1) != win:
            raise TestFail("%s - Unexpected window rendered - \"%s\" != exp \"%s\"" % (name, line.group(1), win,))
    
def _startapp(instance, path, timeout=5):
    instance.wait_startapp(path, timeout)
    
def _mouseto(instance, name, x,y):
    instance.mouse_to(x,y)
    test_assert(name+" mouse(%i,%i) reached" % (x,y,), instance.wait_for_line("windows\] - CursorPos::update - \(%i,%i\)" % (x,y), timeout=3))
    test_assert(name+" mouse(%i,%i) idle" % (x,y,), instance.wait_for_idle(timeout=5))
#def _mouseclick_at(instance, name, x,y btn):
#    _mouseto(instance, name, x,y)
#    instance.mouse_press(btn) 
def _mouseclick(instance, name, btn):
    instance.mouse_press(btn) 
    test_assert("%s mouse(%i down) event" % (name, btn,), instance.wait_for_line("syscalls\] - USER> Window::handle_event\(ev=MouseDown\(\d+, \d+, %i\)\)" % (btn-1,), timeout=3))
    test_assert(name+" mouse(%i down) idle" % (btn,), instance.wait_for_idle(timeout=5))
    time.sleep(0.5)
    instance.mouse_release(btn) 
    test_assert("%s mouse(%i up) event" % (name, btn,), instance.wait_for_line("syscalls\] - USER> Window::handle_event\(ev=MouseUp\(\d+, \d+, %i\)\)" % (btn-1,), timeout=3))
    test_assert(name+" mouse(%i up) idle" % (btn,), instance.wait_for_idle(timeout=5))

#
#
#
def test(instance):
    test_assert("Kernel image start timed out", instance.wait_for_line("OK43e6H", timeout=30))
    instance.start_capture()
    test_assert("Init load timed out", instance.wait_for_line("Entering userland at 0x[0-9a-f]+ '/sysroot/bin/loader' '/sysroot/bin/init'", timeout=10))
    _startapp(instance, "/sysroot/bin/login", timeout=5)
    _gui_rerender(instance, "Login window render", ["Login"])

    test_assert("Initial startup timed out", instance.wait_for_idle(timeout=25))
    instance.screenshot('Login')

    instance.type_string('root')
    while instance.wait_for_idle():
        pass
    _keypress(instance, 'ret', "Username")
    # TODO: Have an item in the log here
    
    instance.type_string('password')
    # - Wait until there's 1s with no action
    while instance.wait_for_idle():
        pass
    _keypress(instance, 'ret', "Password", idle=False)
    _startapp(instance, "/sysroot/bin/handle_server", timeout=10)
    _startapp(instance, "/sysroot/bin/shell", timeout=10)
    _gui_rerender(instance, "Shell idle render", ["Background","SystemBar"])
    test_assert("Shell idle timeout", instance.wait_for_idle(timeout=5))
    instance.screenshot('Shell')
    # TODO: Have an item in the log here

    # >>> Spawn the GUI terminal
    if False:
        # - Open the "System" menu (press left windows key)
        _keypress(instance, 'meta_l', "System menu")
        instance.screenshot('Menu')

        # - Select the top item to open the CLI
        _keypress(instance, 'ret', "CLI Startup", idle=False)
        _startapp(instance, "/sysroot/bin/simple_console", timeout=10)
        test_assert("CLI window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'Console'", timeout=5))
        test_assert("CLI idle timeout", instance.wait_for_idle(timeout=3))
        instance.screenshot('CLI')

        # - Run a command
        instance.type_string('ls /system')
        while instance.wait_for_idle():
            pass
        instance.type_string('/Tifflin/bin')
        while instance.wait_for_idle():
            pass
        _keypress(instance, 'ret', "Run `ls`")
        instance.screenshot('ls')

        # - Quit shell
        instance.type_string('exit')
        while instance.wait_for_idle():
            pass
        instance.type_key('ret')
        test_assert("`exit` return release timeout", instance.wait_for_line("\[syscalls\] - USER> Window::handle_event\(ev=KeyUp\(Return\)\)", timeout=1)) # Release
        test_assert("`exit` reap", instance.wait_for_line("Reaping thread 0x[0-9a-f]+\(\d+ /sysroot/bin/simple_console#1\)", timeout=2))
        instance.screenshot('exit')
        # DISABLED: Idle triggers reaping
        #test_assert("`ls` idle timeout", instance.wait_for_idle(timeout=5))
        
        # - Ensure that the GUI re-renders, and that the terminal no-longer shows
        test_assert("final render", instance.wait_for_line("WindowGroup::redraw: render_order=\[\(1, \[\]\), \(4, \[\(0,20 \+ \d+x\d+\)\]\), \(5, \[\(0,0 \+ \d+x20\)\]\)\]", timeout=5))
        test_assert("Final render idle", instance.wait_for_idle(timeout=5))
    
    # >>> Start the filesystem browser
    if False:
        _keypress(instance, 'meta_l', "Menu 2")
        _keypress(instance, 'down', "Menu 2")
        _keypress(instance, 'down', "Menu 2")
        _keypress(instance, 'ret', "Menu 2", idle=False)
        test_assert("FileBrowser window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'File browser'", timeout=5))
        test_assert("FileBrowser idle timeout", instance.wait_for_idle(timeout=3))
        instance.screenshot('FileBrowser')
        
        _keypress(instance, 'down', "File browser")
        _keypress(instance, 'down', "File browser")
        #test_assert("FileBrowser window render", instance.wait_for_idle(timeout=5))
        instance.screenshot('FileBrowser-sel2')
        #_keypress(instance, 'up', "File browser")
        _keypress(instance, 'ret', "File browser", idle=False)
        test_assert("FileBrowser window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'File browser'", timeout=5))
        test_assert("FileBrowser window render", instance.wait_for_idle(timeout=5))
        instance.screenshot('FileBrowser-enter2')
        
        # Close using alt-f4
        instance.type_combo(['alt', 'f4'])
        test_assert("Close FileBrowser alt press timeout", instance.wait_for_line("\[syscalls\] - USER> Window::handle_event\(ev=KeyDown\(LeftAlt\)\)", timeout=1))
        test_assert("Close FileBrowser f4 press timeout", instance.wait_for_line("\[syscalls\] - USER> Window::handle_event\(ev=KeyDown\(F4\)\)", timeout=1))
        test_assert("Close FileBrowser f4 release timeout", instance.wait_for_line("\[syscalls\] - USER> Window::handle_event\(ev=KeyUp\(F4\)\)", timeout=1))
        test_assert("Close FileBrowser reap", instance.wait_for_line("Reaping thread 0x[0-9a-f]+\(\d+ /sysroot/bin/filebrowser#1\)", timeout=2))
        test_assert("Close FileBrowser idle", instance.wait_for_idle(timeout=10))
    # >>> Start the filesystem browser (again)
    if True:
        _keypress(instance, ['meta_l', 'e'], "FileBrowser")
        test_assert("FileBrowser window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'File browser'", timeout=5))
        test_assert("FileBrowser idle timeout", instance.wait_for_idle(timeout=3))
        instance.screenshot('FileBrowser2')

        _keypress(instance, 'down', "File browser")
        _keypress(instance, 'down', "File browser")
        _keypress(instance, 'ret', "File browser")
        instance.screenshot('FileBrowser2-1')
        _keypress(instance, 'down', "File browser")
        _keypress(instance, 'ret', "File browser", idle=False)
        instance.screenshot('FileBrowser2-2')
        #_matchline(instance, "FileViewer open file", "openfile\(Path\(b\"([^\"]+)\"\)", ["/system/1.txt"], timeout=5)
        _startapp(instance, "/sysroot/bin/fileviewer", timeout=5)
        _gui_rerender(instance, "Close FileBrowser", ['File viewer'])
        #test_assert("FileViewer window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'File viewer'", timeout=5))
        test_assert("FileViewer idle timeout", instance.wait_for_idle(timeout=3))
        instance.screenshot('Browser-Viewer')
        _keypress(instance, ['alt', 'f4'], "FileViewerClose")
        test_assert("Close FileViewer reap", instance.wait_for_line("Reaping thread 0x[0-9a-f]+\(\d+ /sysroot/bin/fileviewer#1\)", timeout=2))
        _gui_rerender(instance, "Close FileBrowser", ['Login','Background','SystemBar','File browser'])
        test_assert("FileViewer reap idle timeout", instance.wait_for_idle(timeout=15))

        # - By default, WTK creates 250x200 windows at (150,100)
        _mouseto(instance, "Exit button", 150 + 250 - 6, 100 + 6)
        instance.screenshot('mouse')
        time.sleep(1)
        _mouseclick(instance, "Exit button", 1)
        test_assert("Close FileBrowser reap", instance.wait_for_line("Reaping thread 0x[0-9a-f]+\(\d+ /sysroot/bin/filebrowser#1\)", timeout=2))
        #_gui_rerender(instance, "Close FileBrowser", ['Login','Background','SystemBar'])
        test_assert("Close FileBrowser idle", instance.wait_for_idle(timeout=10))

    # >>> Start the text viewer
    if True:
        _keypress(instance, 'meta_l', "Menu 2")
        _keypress(instance, 'down', "Menu 2")
        _keypress(instance, 'down', "Menu 2")
        _keypress(instance, 'down', "Menu 2")
        instance.screenshot('FileViewer-P')
        _keypress(instance, 'ret', "Menu 2", idle=False)
        _startapp(instance, "/sysroot/bin/fileviewer", timeout=5)
        test_assert("FileViewer window render", instance.wait_for_line("\[gui::windows\] - L\d+: WindowGroup::redraw: \d+ 'File viewer'", timeout=5))
        test_assert("FileViewer idle timeout", instance.wait_for_idle(timeout=3))
        instance.screenshot('FileViewer')
        

    while instance.wait_for_idle(timeout=3):
        pass


TestInstance.run_test("amd64", "CLI",  test)
