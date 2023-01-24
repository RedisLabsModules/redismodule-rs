#!/usr/bin/env python3

import sys
import os
import argparse

HERE = os.path.abspath(os.path.dirname(__file__))
ROOT = os.path.abspath(os.path.join(HERE, ".."))
READIES = os.path.join(ROOT, "deps/readies")
sys.path.insert(0, READIES)
import paella

#----------------------------------------------------------------------------------------------

class RedisModuleRSSetup(paella.Setup):
    def __init__(self, args):
        paella.Setup.__init__(self, args.nop)

    def common_first(self):
        self.install_downloaders()

        self.install("git")

        self.run("%s/bin/enable-utf8" % READIES)

        self.run("%s/bin/getclang --modern" % READIES)
        self.run("%s/bin/getrust" % READIES)

        if self.osnick == 'ol8':
            self.install('tar')
        self.run("%s/bin/getcmake --usr" % READIES)

    def debian_compat(self):
        self.run("%s/bin/getgcc" % READIES)

    def redhat_compat(self):
        self.install("redhat-lsb-core")
        self.run("%s/bin/getgcc --modern" % READIES)

        if not self.platform.is_arm():
            self.install_linux_gnu_tar()

    def fedora(self):
        self.run("%s/bin/getgcc" % READIES)

    def macos(self):
        self.install_gnu_utils()
        self.run("%s/bin/getredis -v 6" % READIES)

    def common_last(self):
        self.pip_install("toml")

#----------------------------------------------------------------------------------------------

parser = argparse.ArgumentParser(description='Set up system for build.')
parser.add_argument('-n', '--nop', action="store_true", help='no operation')
args = parser.parse_args()

RedisModuleRSSetup(args).setup()
