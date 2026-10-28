// Fetch and verify the official game for a temporary, mod-free fallback.
using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Runtime.Serialization;
using System.Text.RegularExpressions;

namespace SkateShared {
 [DataContract] sealed class OfficialAsset {
  [DataMember(Name="name")] public string Name=null;
  [DataMember(Name="size")] public long Size=0;
  [DataMember(Name="digest")] public string Digest=null;
  [DataMember(Name="browser_download_url")] public string Address=null;
 }
 [DataContract] sealed class OfficialRelease {
  [DataMember(Name="tag_name")] public string Tag=null;
  [DataMember(Name="draft")] public bool Draft=false;
  [DataMember(Name="prerelease")] public bool Prerelease=false;
  [DataMember(Name="assets")] public OfficialAsset[] Assets=null;
 }
 public static class OfficialFallback {
  const string Api="https://api.github.com/repos/skate6743/skate2-rust-port/releases/latest";
  const string DownloadPrefix="https://github.com/skate6743/skate2-rust-port/releases/download/";
  const long MaxArchive=128L*1024*1024, MaxExe=256L*1024*1024;

  // Stages a verified EXE only; it never changes the installed game or saves.
  public static string Prepare(string root, string expectedVersion, ITransport network, Action<string> progress) {
   Version expected=OfficialVersion.Parse(expectedVersion);
   var release=Codec.Read<OfficialRelease>(network.Get(new Uri(Api),1024*1024));
   if(release==null || release.Draft || release.Prerelease || release.Tag==null ||
      !Regex.IsMatch(release.Tag,@"\Av\d{1,5}\.\d{1,5}\.\d{1,5}\z") ||
      !OfficialVersion.Parse(release.Tag.Substring(1)).Equals(expected))
    throw new InvalidDataException("The official release does not match the current game version.");
   var matches=(release.Assets ?? new OfficialAsset[0]).Where(a=>a!=null && a.Name=="skate2-windows-x64.zip").ToArray();
   if(matches.Length!=1) throw new InvalidDataException("The official Windows download is unavailable.");
   var asset=matches[0];
   string prefix=DownloadPrefix+release.Tag+"/";
   Uri address;
   if(asset.Size<=0 || asset.Size>MaxArchive || !Regex.IsMatch(asset.Digest ?? "",@"\Asha256:[a-f0-9]{64}\z") ||
      !Uri.TryCreate(asset.Address,UriKind.Absolute,out address) || address.Scheme!="https" ||
      !String.Equals(asset.Address,prefix+asset.Name,StringComparison.Ordinal) || address.Query!="" || address.Fragment!="")
    throw new InvalidDataException("The official download details could not be verified.");
   string folder=Path.Combine(root,"smokey-official-cache");Codec.GuardDirectory(folder);
   string archive=Path.Combine(folder,".official-"+Guid.NewGuid().ToString("N")+".zip");
   string stage=Path.Combine(folder,".official-"+Guid.NewGuid().ToString("N")+".new");
   try {
    progress("Downloading the official game so you can keep playing online...");
    network.Download(address,archive,asset.Size,delegate(long n){});
    if(Codec.HashFile(archive)!=asset.Digest.Substring(7)) throw new InvalidDataException("The official download failed its integrity check.");
    using(var zip=ZipFile.OpenRead(archive)) {
     var entries=zip.Entries.Where(e=>Path.GetFileName(e.FullName.Replace('\\','/'))=="skate2-pc.exe").ToArray();
     if(entries.Length!=1 || entries[0].Length<2 || entries[0].Length>MaxExe)
      throw new InvalidDataException("The official game archive has an unexpected layout.");
     using(var input=entries[0].Open()) using(var output=new FileStream(stage,FileMode.CreateNew,FileAccess.Write,FileShare.None)) {
      byte[] bytes=new byte[128*1024];long total=0;int n;
      while((n=input.Read(bytes,0,bytes.Length))!=0) { total=checked(total+n);if(total>MaxExe)throw new InvalidDataException("Official game exceeds its size limit.");output.Write(bytes,0,n); }
      if(total!=entries[0].Length)throw new InvalidDataException("Official game download is incomplete.");output.Flush(true);
     }
    }
    using(var exe=File.OpenRead(stage)) if(exe.ReadByte()!=77 || exe.ReadByte()!=90) throw new InvalidDataException("Official download is not a Windows executable.");
    string hash=Codec.HashFile(stage);string final=Path.Combine(folder,"official-"+expectedVersion+"-"+hash.Substring(0,12)+".exe");
    if(File.Exists(final)) { if(Codec.HashFile(final)!=hash)throw new IOException("Official game cache failed verification.");File.Delete(stage); }
    else File.Move(stage,final);
    return final;
   } finally { if(File.Exists(stage))File.Delete(stage);if(File.Exists(archive))File.Delete(archive); }
  }
 }
}
