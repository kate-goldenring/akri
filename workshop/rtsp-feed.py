#!/usr/bin/env python
# Run privileged: `sudo /usr/bin/python3 /home/.../workshop/rtsp-feed.py`

import sys
import gi

gi.require_version('Gst', '1.0')
gi.require_version('GstRtspServer', '1.0')
from gi.repository import Gst, GstRtspServer, GObject, GLib

loop = GLib.MainLoop()
Gst.init(None)

class TestRtspMediaFactory(GstRtspServer.RTSPMediaFactory):
    def __init__(self):
        GstRtspServer.RTSPMediaFactory.__init__(self)

    def do_create_element(self, url):
        # Set mp4 file path to filesrc's location property
        src_demux = "filesrc location=/home/kagold/akri-fork/workshop/akri.mp4 ! qtdemux name=demux"
        h264_transcode = "demux.video_0"
        pipeline = "{0} {1} ! queue ! rtph264pay name=pay0 config-interval=1 pt=96".format(src_demux, h264_transcode)
        mock_pipeline = "videotestsrc pattern=bar horizontal-speed=2 background-color=9228238 foreground-color=4080751 ! x264enc ! queue ! rtph264pay name=pay0 config-interval=1 pt=96" 
        print ("Element created: " + mock_pipeline)
        return Gst.parse_launch(mock_pipeline)

class GstreamerRtspServer():
    def __init__(self):
        self.rtspServer = GstRtspServer.RTSPServer()
        factory = TestRtspMediaFactory()
        factory.set_shared(True)
        mountPoints = self.rtspServer.get_mount_points()
        mountPoints.add_factory("/stream1", factory)
        self.rtspServer.attach(None)

if __name__ == '__main__':
    s = GstreamerRtspServer()
    loop.run()