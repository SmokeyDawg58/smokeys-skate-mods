// The usual game icon starts this only while a temporary official fallback is active.
using System;
using System.Diagnostics;
using System.IO;
using System.Windows.Forms;

namespace SkateShared {
 static class FallbackLauncher {
  [STAThread] static int Main(string[] args) {
   string root=AppDomain.CurrentDomain.BaseDirectory;
   try {
    try {
      string updater=FallbackInstall.VerifiedUpdater(root);
      var next=Process.Start(new ProcessStartInfo(updater,"--fallback-launch "+Process.GetCurrentProcess().Id) { WorkingDirectory=root,UseShellExecute=true });
      if(next!=null) { next.Dispose();return 0; }
    } catch(Exception) { /* A verified official game can still be played. */ }
    string official=FallbackInstall.VerifiedOfficial(root);
    MessageBox.Show("Smokey's updater is unavailable. The official game will open without mods.","Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);
    var game=Process.Start(new ProcessStartInfo(official) { WorkingDirectory=root,UseShellExecute=true });
    if(game==null)throw new IOException("Windows could not open the official game.");game.Dispose();return 0;
   } catch(Exception error) {
    MessageBox.Show(error.Message,"Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);return 1;
   }
  }
 }
}
