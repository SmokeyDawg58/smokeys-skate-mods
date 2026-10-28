using System;
using System.IO;
using System.Linq;
using System.Text;
namespace SkateShared {
 sealed class FakeTransport : ITransport {
  readonly string folder;public byte[] Signed;public string Official="0.2.8";
  public FakeTransport(string path,byte[] signed){folder=path;Signed=signed;}
  public byte[] Get(Uri uri,long limit){if(uri.Host=="github.com")return Encoding.UTF8.GetBytes("skate2 release 1\nversion "+Official+"\n");return Signed;}
  public void Download(Uri uri,string path,long size,Action<long> progress){string source=Path.Combine(folder,Path.GetFileName(uri.AbsolutePath));File.Copy(source,path);if(new FileInfo(path).Length!=size)throw new Exception("Wrong part size");progress(size);}
 }
 static class SmokeyTests {
  static void Assert(bool value,string message){if(!value)throw new Exception(message);}
  public static int Main(string[] args){try{
   string root=args[0],feed=args[1];Directory.CreateDirectory(root);Directory.CreateDirectory(Path.Combine(root,"saves"));
   byte[] original=Encoding.ASCII.GetBytes("MZ-original-pre-mod-game");File.WriteAllBytes(Path.Combine(root,"skate2-pc.exe"),original);File.WriteAllText(Path.Combine(root,"saves","SKATER.P"),"keep-save");File.WriteAllText(Path.Combine(root,"gameplay.json"),"keep-settings");
   SmokeyUpdater.EnsureChannel(root);var network=new FakeTransport(feed,File.ReadAllBytes(Path.Combine(feed,"latest.json")));var u=new Updater(root,Trust.PublicKey,network,delegate(string s){});
   Assert(u.State.Current==0,"fresh state");Assert(UpdateFlow.Run(root,u,network,true),"first install");
   string ownerSettings=Path.Combine(root,DemandBuilder.SettingsName);
   File.WriteAllText(ownerSettings,"{\"python\":\"missing\",\"script\":\"missing\",\"config\":\"missing\"}");
   string beforeFailedBuild=Codec.HashFile(Path.Combine(root,"skate2-pc.exe"));network.Official="0.2.9";
   bool mismatchAfterBuildFailure=false;try{UpdateFlow.Run(root,u,network,true);}catch(GameVersionMismatchException){mismatchAfterBuildFailure=true;}
   Assert(mismatchAfterBuildFailure,"failed builder still offers official game on mismatch");
   Assert(Codec.HashFile(Path.Combine(root,"skate2-pc.exe"))==beforeFailedBuild,"failed builder keeps installed game");
   network.Official="0.2.8";bool matchingBuildFailure=false;try{UpdateFlow.Run(root,u,network,true);}catch(IOException){matchingBuildFailure=true;}
   Assert(matchingBuildFailure,"matching-version builder failure is reported");File.Delete(ownerSettings);
   Assert(SafeRestore.VerifiedOriginal(root)!=null,"backup exists");Assert(File.ReadAllText(Path.Combine(root,"saves","SKATER.P"))=="keep-save","save preserved");Assert(File.ReadAllText(Path.Combine(root,"gameplay.json"))=="keep-settings","settings preserved");Assert(SmokeyUpdater.CheckOnly(root,network)==0,"current no prompt");network.Official="0.2.9";Assert(SmokeyUpdater.CheckOnly(root,network)==11,"new game update requires matching mod");network.Official="0.2.8";
   network.Signed=File.ReadAllBytes(args[2]);Assert(SmokeyUpdater.CheckOnly(root,network)==10,"new signed release notified");
   byte[] current=File.ReadAllBytes(Path.Combine(root,"skate2-pc.exe"));string hash=Codec.HashBytes(current);
   File.WriteAllBytes(Path.Combine(root,"skate2-pc.exe"),Encoding.ASCII.GetBytes("MZ-changed-by-user"));network.Official="0.2.9";Assert(SmokeyUpdater.CheckOnly(root,network)==11,"unverified preview explains newer game release");network.Official="0.2.8";Assert(SmokeyUpdater.CheckOnly(root,network)==2,"unverified executable never advertises installation");bool refused=false;try{SafeRestore.Restore(root,SafeRestore.VerifiedOriginal(root));}catch(IOException){refused=true;}Assert(refused,"changed executable refused");Assert(Encoding.ASCII.GetString(File.ReadAllBytes(Path.Combine(root,"skate2-pc.exe")))=="MZ-changed-by-user","changed executable kept");
   File.WriteAllBytes(Path.Combine(root,"skate2-pc.exe"),current);Assert(Codec.HashFile(Path.Combine(root,"skate2-pc.exe"))==hash,"restore fixture");SafeRestore.Restore(root,SafeRestore.VerifiedOriginal(root));Assert(File.ReadAllBytes(Path.Combine(root,"skate2-pc.exe")).SequenceEqual(original),"exact prior game restored");Assert(File.ReadAllText(Path.Combine(root,"saves","SKATER.P"))=="keep-save","uninstall preserves save");Assert(SmokeyUpdater.CheckOnly(root,network)!=10,"restored game is not re-prompted");
   string old=Path.Combine(root,"old-client");Directory.CreateDirectory(old);File.WriteAllText(Path.Combine(old,"manual-update.json"),"{}");File.WriteAllBytes(Path.Combine(old,"skate2-pc.exe"),original);SafeRestore.RememberPreModGame(old);Assert(SafeRestore.VerifiedOriginal(old)==null,"no invented original backup for legacy users");
   string refresh=Path.Combine(root,"helper-refresh");Directory.CreateDirectory(refresh);
   string primary=Path.Combine(refresh,"Smokey's Skate Mods Updater.exe");File.WriteAllText(primary,"old owned helper");
   string marker=Path.Combine(refresh,"smokey-updater-version.txt");File.WriteAllText(marker,"1:"+Codec.HashFile(primary));
   byte[] helperBytes=Encoding.ASCII.GetBytes("MZ new versioned helper");string helperHash=Codec.HashBytes(helperBytes);
   string versioned=Path.Combine(refresh,"Smokey's Skate Mods Updater-"+helperHash.Substring(0,8)+".exe");File.WriteAllBytes(versioned,helperBytes);
   SmokeyUpdater.RefreshOwnedPrimary(refresh,versioned);Assert(Codec.HashFile(primary)==helperHash,"known old helper refreshed");
   Assert(File.ReadAllText(marker)=="1:"+helperHash,"helper ownership updated");
   File.WriteAllText(primary,"unrelated user file");SmokeyUpdater.RefreshOwnedPrimary(refresh,versioned);Assert(File.ReadAllText(primary)=="unrelated user file","unrecognized helper kept");
   string fallback=Path.Combine(root,"fallback-client");Directory.CreateDirectory(fallback);Directory.CreateDirectory(Path.Combine(fallback,"saves"));
   File.WriteAllBytes(Path.Combine(fallback,"skate2-pc.exe"),original);File.WriteAllText(Path.Combine(fallback,"saves","SKATER.P"),"keep-save");
   SmokeyUpdater.EnsureChannel(fallback);var fallbackNet=new FakeTransport(feed,File.ReadAllBytes(Path.Combine(feed,"latest.json")));
   var first=new Updater(fallback,Trust.PublicKey,fallbackNet,delegate(string s){});Assert(UpdateFlow.Run(fallback,first,fallbackNet,true),"initial fallback client install");
   string stock=Path.Combine(fallback,"verified-stock.exe");File.WriteAllBytes(stock,Encoding.ASCII.GetBytes("MZ official fixture"));
   const string helper="Smokey's Skate Mods Updater.exe";File.WriteAllBytes(Path.Combine(fallback,helper),Encoding.ASCII.GetBytes("MZ updater fixture"));
   FallbackInstall.Install(fallback,stock,FallbackMode.VerifiedInstalledModHash(fallback),"0.2.8",Encoding.ASCII.GetBytes("MZ launcher fixture"),helper);
   Assert(FallbackInstall.IsActiveLauncher(fallback),"fallback launcher active");
   fallbackNet.Signed=File.ReadAllBytes(args[2]);Assert(FallbackMode.CheckForMod(fallback,fallbackNet,SmokeyUpdater.Feed,"0.2.8"),"compatible signed update offered");
   var returning=new Updater(fallback,Trust.PublicKey,fallbackNet,delegate(string s){});Assert(UpdateFlow.Run(fallback,returning,fallbackNet,true),"signed mod restored");
   FallbackInstall.ClearAfterModInstall(fallback,returning);Assert(!File.Exists(Path.Combine(fallback,FallbackInstall.ReceiptName)),"fallback ended after verified restore");
   ManualInstall.VerifyCurrent(fallback,returning);Assert(File.ReadAllText(Path.Combine(fallback,"saves","SKATER.P"))=="keep-save","fallback return preserves save");
   Console.WriteLine("PASS signed fresh install, update check, safe restore and preservation");return 0;
  }catch(Exception e){Console.Error.WriteLine(e);return 1;}}
 }
}
