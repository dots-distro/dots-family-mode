#!/usr/bin/env python3
import dbus
import dbus.service
import dbus.mainloop.glib
from gi.repository import GLib
import sys

class TestService(dbus.service.Object):
    def __init__(self):
        bus_name = dbus.service.BusName('org.dots.FamilyDaemon', bus=dbus.SessionBus())
        super().__init__(bus_name, '/org/dots/FamilyDaemon')
        print("Successfully claimed D-Bus name: org.dots.FamilyDaemon")

if __name__ == '__main__':
    dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)
    try:
        service = TestService()
        print("D-Bus service running. Press Ctrl+C to stop.")
        loop = GLib.MainLoop()
        loop.run()
    except dbus.exceptions.NameExistsException:
        print("ERROR: D-Bus name org.dots.FamilyDaemon is already claimed")
        sys.exit(1)
    except Exception as e:
        print(f"ERROR: Failed to claim D-Bus name: {e}")
        sys.exit(1)