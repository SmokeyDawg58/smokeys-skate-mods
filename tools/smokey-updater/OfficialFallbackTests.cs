using System;
using System.IO;
using System.IO.Compression;
using System.Linq;
namespace SkateShared {
 sealed class OfficialTestTransport:ITransport {
  public byte[] Metadata; public string Archive;
  public byte[] Get(Uri uri,long limit) { return Metadata; }
  public void Download(Uri uri,string path,long size,Action<long> progress) { File.Copy(Archive,path);progress(size); }
 }
 static class OfficialFallbackTests {
  static void Check(bool ok,string message) { if(!ok)throw new Exception(message); }
  public static int Main(string[] args) { try {
   string root=args[0];Directory.CreateDirectory(root);
   string game=Path.Combine(root,"skate2-pc.exe");File.WriteAllText(game,"existing game");
   string save=Path.Combine(root,"save.txt");File.WriteAllText(save,"existing save");
   string archive=Path.Combine(root,"fixture.zip");
   using(var zip=ZipFile.Open(archive,ZipArchiveMode.Create)) using(var stream=zip.CreateEntry("game/skate2-pc.exe").Open()) {
    byte[] bytes=System.Text.Encoding.ASCII.GetBytes("MZ verified official game");stream.Write(bytes,0,bytes.Length);
   }
   var asset=new OfficialAsset { Name="skate2-windows-x64.zip",Size=new FileInfo(archive).Length,
    Digest="sha256:"+Codec.HashFile(archive),Address="https://github.com/skate6743/skate2-rust-port/releases/download/v0.2.9/skate2-windows-x64.zip" };
   var release=new OfficialRelease { Tag="v0.2.9",Assets=new[]{asset} };
   var network=new OfficialTestTransport { Metadata=Codec.Json(release),Archive=archive };
   string staged=OfficialFallback.Prepare(root,"0.2.9",network,delegate(string s){});
   Check(File.ReadAllText(staged)=="MZ verified official game","verified official EXE staged");
   Check(File.ReadAllText(game)=="existing game" && File.ReadAllText(save)=="existing save","existing files unchanged");
   asset.Digest="sha256:"+new string('0',64);network.Metadata=Codec.Json(release);
   bool refused=false;try { OfficialFallback.Prepare(root,"0.2.9",network,delegate(string s){}); }catch(InvalidDataException){refused=true;}
   Check(refused,"wrong digest rejected");
   refused=false;try { OfficialFallback.Prepare(root,"0.3.0",network,delegate(string s){}); }catch(InvalidDataException){refused=true;}
   Check(refused,"wrong version rejected");
   Check(File.ReadAllText(game)=="existing game" && File.ReadAllText(save)=="existing save","failure preserves files");
   var old=new ManualReceipt { Sequence=41 };var state=new State { Current=41,Highest=41 };
   var mod=new Release { Sequence=42,GameVersion="0.2.8" };
   Check(!FallbackMode.ShouldOffer(mod,old,state,"0.2.9"),"outdated mod not offered");
   mod.GameVersion="0.2.9";Check(FallbackMode.ShouldOffer(mod,old,state,"0.2.9"),"compatible update offered");
   mod.GameVersion="0.3.0";Check(!FallbackMode.ShouldOffer(mod,old,state,"0.2.9"),"future incompatible mod not offered");mod.GameVersion="0.2.9";
   state.Blocked.Add(42);Check(!FallbackMode.ShouldOffer(mod,old,state,"0.2.9"),"blocked release not offered");
   byte[] launcher=System.Text.Encoding.ASCII.GetBytes("MZ temporary launcher");
   const string updater="Smokey's Skate Mods Updater.exe";File.WriteAllText(Path.Combine(root,updater),"MZ updater");
   FallbackInstall.Install(root,staged,Codec.HashFile(game),"0.2.9",launcher,updater);
   Check(File.ReadAllBytes(game).SequenceEqual(launcher),"temporary launcher installed");
   Check(FallbackInstall.IsActiveLauncher(root),"launcher state recognized");
   Check(File.ReadAllText(FallbackInstall.VerifiedOfficial(root))=="MZ verified official game","verified official game retained");
   Check(File.ReadAllText(save)=="existing save","fallback preserves save");
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(Path.Combine(root,FallbackInstall.ReceiptName)));
   Check(File.ReadAllText(Path.Combine(root,"mod-update-backups",receipt.BackupFile))=="existing game","mod backed up");
   File.WriteAllText(game,"changed game");refused=false;try { FallbackInstall.IsActiveLauncher(root); }catch(IOException){refused=true;}
   Check(refused,"changed game icon is never overwritten");File.WriteAllBytes(game,launcher);
   File.AppendAllText(Path.Combine(root,receipt.OfficialFile),"changed");
   refused=false;try { FallbackInstall.VerifiedOfficial(root); }catch(IOException){refused=true;}
   Check(refused,"tampered official game rejected");
   FallbackInstall.UpdateOfficial(root,staged,"0.2.9");
   Check(File.ReadAllText(FallbackInstall.VerifiedOfficial(root))=="MZ verified official game","official game repaired without replacing changed file");
   if(args.Length>=3) {
    string launchRoot=Path.Combine(root,"launch-fixture");Directory.CreateDirectory(launchRoot);
    string target=Path.Combine(launchRoot,"skate2-pc.exe");File.WriteAllText(target,"MZ existing mod");
    string stock=Path.Combine(launchRoot,"verified-stock.exe");File.WriteAllText(stock,"MZ verified stock");
    File.Copy(args[2],Path.Combine(launchRoot,updater));
    FallbackInstall.Install(launchRoot,stock,Codec.HashFile(target),"0.2.9",File.ReadAllBytes(args[1]),updater);
    using(var process=System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(target){WorkingDirectory=launchRoot,UseShellExecute=false,CreateNoWindow=true})) { process.WaitForExit(10000);Check(process.ExitCode==0,"fallback launcher succeeds"); }
    string called=Path.Combine(launchRoot,"launch-called.txt");
    for(int i=0;i<100 && !File.Exists(called);i++)System.Threading.Thread.Sleep(50);
    Check(File.Exists(called) && File.ReadAllText(called).StartsWith("--fallback-launch "),"normal icon launches the updater once");
   }
   if(args.Length==5) {
    string realRoot=Path.Combine(root,"real-archive-fixture");Directory.CreateDirectory(realRoot);
    var realAsset=new OfficialAsset { Name="skate2-windows-x64.zip",Size=new FileInfo(args[3]).Length,
     Digest="sha256:"+Codec.HashFile(args[3]),Address="https://github.com/skate6743/skate2-rust-port/releases/download/v0.2.9/skate2-windows-x64.zip" };
    var realNet=new OfficialTestTransport { Metadata=Codec.Json(new OfficialRelease { Tag="v0.2.9",Assets=new[]{realAsset} }),Archive=args[3] };
    string extracted=OfficialFallback.Prepare(realRoot,"0.2.9",realNet,delegate(string s){});
    Check(Codec.HashFile(extracted)==args[4],"real official archive yields the previously verified EXE");
   }
   Console.WriteLine("PASS verified official fallback preparation, digest, version, and preservation");return 0;
  } catch(Exception e){Console.Error.WriteLine(e);return 1;} }
 }
}
