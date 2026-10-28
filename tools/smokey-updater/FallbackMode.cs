using System;
using System.IO;
using System.Linq;

namespace SkateShared {
 public static class FallbackMode {
  public static string VerifiedInstalledModHash(string root) {
   var installed=Codec.Read<ManualReceipt>(File.ReadAllBytes(Path.Combine(root,"manual-update.json")));
   if(installed==null || installed.Sequence<1)throw new InvalidDataException("The installed mod record is invalid.");
   string manifest=Path.Combine(root,"versions",installed.Sequence.ToString("D12"),"release.json");
   var release=Codec.Verify(File.ReadAllBytes(manifest),Trust.PublicKey);
   var file=release.Files.Single(item=>item.Name=="skate2-pc.exe");
   if(release.Sequence!=installed.Sequence || installed.Hash!=file.Hash || installed.GameVersion!=release.GameVersion)
    throw new InvalidDataException("The installed mod record could not be verified.");
   Codec.VerifyFile(ManualInstall.GamePath(root),file);return file.Hash;
  }
  // Call only with a verified, signed mod release.
  public static bool ShouldOffer(Release latest,ManualReceipt installed,State state,string officialVersion) {
   if(latest==null || installed==null || state==null || latest.Sequence<=installed.Sequence ||
      latest.Sequence<state.Highest || (state.Blocked!=null && state.Blocked.Contains(latest.Sequence)))return false;
   return OfficialVersion.Parse(latest.GameVersion).CompareTo(OfficialVersion.Parse(officialVersion))==0;
  }
  public static bool CheckForMod(string root,ITransport network,string feed,string officialVersion) {
   var installed=Codec.Read<ManualReceipt>(File.ReadAllBytes(Path.Combine(root,"manual-update.json")));
   var state=Codec.Read<State>(File.ReadAllBytes(Path.Combine(root,"update-state.json")));
   var channel=new Uri(feed,UriKind.Absolute);
   if(channel.Scheme!="https" || !String.IsNullOrEmpty(channel.UserInfo) || !String.IsNullOrEmpty(channel.Query) || !String.IsNullOrEmpty(channel.Fragment))
    throw new InvalidDataException("The mod update address is invalid.");
   var latest=Codec.Verify(network.Get(new Uri(channel.AbsoluteUri+"?check="+DateTime.UtcNow.Ticks),Codec.MaxManifest),Trust.PublicKey);
   return ShouldOffer(latest,installed,state,officialVersion);
  }
 }
}
