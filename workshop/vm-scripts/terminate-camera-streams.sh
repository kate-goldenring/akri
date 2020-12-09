#!/bin/bash

# Ask gst-launch nicely to terminate.
if pgrep gst-launch-1.0 > /dev/null; then
            sudo pkill gst-launch-1.0
fi

# Forcibly terminate.
if pgrep gst-launch-1.0 > /dev/null; then
            sudo pkill -9 gst-launch-1.0
fi