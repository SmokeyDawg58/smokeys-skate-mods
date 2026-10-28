using System;
using System.Drawing;
using System.IO;
using System.Linq;
using System.Diagnostics;
using System.Runtime.Serialization;
using System.Threading.Tasks;
using System.Windows.Forms;

namespace SkateShared {
 [DataContract] sealed class Notes { [DataMember(Name="release_notes")] public string[] ReleaseNotes = new string[0]; }
 static class SmokeyUpdater {
  internal const string Feed="https://regal-clafoutis-04339e.netlify.app/latest.json";
  internal static string Root {get {return AppDomain.CurrentDomain.BaseDirectory;}}
  [STAThread] static int Main(string[] args) {
   string root=Root;
   bool resumeGame=args.Length==2 && args[0]=="--after-game";
   bool fallbackGame=args.Length==2 && args[0]=="--fallback-launch";
   if(args.Length==0 && File.Exists(Path.Combine(root,FallbackInstall.ReceiptName))) {
    try { fallbackGame=FallbackInstall.IsActiveLauncher(root); }
    catch(Exception error){MessageBox.Show(error.Message,"Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);return 1;}
   }
   if (args.Length==1 && args[0]=="--check-only") {try {RefreshOwnedPrimary(root,Application.ExecutablePath);}catch {}try {return CheckOnly(root,new HttpsTransport());}catch {return 2;}}
   if (args.Length==2 && (resumeGame || fallbackGame)) {
    int id;if(!int.TryParse(args[1],out id)||id<=0)return 2;
    try {using(var parent=Process.GetProcessById(id)){if(!parent.WaitForExit(60000))return 2;}}catch(ArgumentException){}
   } else if (args.Length==2 && args[0]=="--render-preview") {
    Application.EnableVisualStyles();using(var form=new SmokeyWindow(root,true)) {form.ShowInTaskbar=false;form.StartPosition=FormStartPosition.Manual;form.Location=new Point(-5000,-5000);form.Show();Application.DoEvents();using(var image=new Bitmap(form.Width,form.Height)){form.DrawToBitmap(image,new Rectangle(0,0,form.Width,form.Height));image.Save(args[1]);}form.Close();}return 0;
   } else if(args.Length!=0) return 2;
   Application.EnableVisualStyles();Application.SetCompatibleTextRenderingDefault(false);
   try { Codec.GuardDirectory(root);using(var lease=new FileStream(Path.Combine(root,"launcher.lock"),FileMode.OpenOrCreate,FileAccess.ReadWrite,FileShare.None)) using(var window=new SmokeyWindow(root,false,resumeGame,fallbackGame)) Application.Run(window);return 0; }
   catch(Exception error){MessageBox.Show(error.Message,"Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);return 1;}
  }
  internal static void StartGame(string root) {
   string game=ManualInstall.GamePath(root);
   if(!File.Exists(game))throw new IOException("The game executable was not found.");
   var launched=Process.Start(new ProcessStartInfo(game){WorkingDirectory=root,UseShellExecute=true});
   if(launched==null)throw new IOException("Windows could not open the game.");
   launched.Dispose();
  }
  internal static void StartOfficial(string root) {
   string game=FallbackInstall.VerifiedOfficial(root);
   var launched=Process.Start(new ProcessStartInfo(game){WorkingDirectory=root,UseShellExecute=true});
   if(launched==null)throw new IOException("Windows could not open the official game.");launched.Dispose();
  }
  internal static byte[] FallbackLauncherBytes() {
   using(var stream=System.Reflection.Assembly.GetExecutingAssembly().GetManifestResourceStream("smokey.fallback_launcher")) {
    if(stream==null)throw new IOException("The temporary game launcher is missing.");
    using(var result=new MemoryStream()){stream.CopyTo(result);return result.ToArray();}
   }
  }
  internal static void EnsureChannel(string root) {
   string path=Path.Combine(root,"update-channel.json");if(!File.Exists(path)) Codec.WriteAtomic(path,Codec.Json(new Channel{FeedUrl=Feed}));
  }
  // Refresh only a helper whose ownership marker still matches its bytes.
  // If Windows has it open, keep using the new versioned helper this launch.
  internal static void RefreshOwnedPrimary(string root,string running) {
   string name=Path.GetFileName(running);
   var match=System.Text.RegularExpressions.Regex.Match(name,@"\ASmokey's Skate Mods Updater-([a-f0-9]{8})\.exe\z");
   if(!match.Success || !String.Equals(Path.GetFullPath(Path.GetDirectoryName(running)).TrimEnd(Path.DirectorySeparatorChar),Path.GetFullPath(root).TrimEnd(Path.DirectorySeparatorChar),StringComparison.OrdinalIgnoreCase))return;
   string primary=Path.Combine(root,"Smokey's Skate Mods Updater.exe"),marker=Path.Combine(root,"smokey-updater-version.txt");
   if(!File.Exists(primary) || !File.Exists(marker) || !File.Exists(running) || new FileInfo(marker).Length>256 ||
      (File.GetAttributes(primary)&FileAttributes.ReparsePoint)!=0 || (File.GetAttributes(marker)&FileAttributes.ReparsePoint)!=0 || (File.GetAttributes(running)&FileAttributes.ReparsePoint)!=0)return;
   string current=Codec.HashFile(running),old=Codec.HashFile(primary),ownership=File.ReadAllText(marker);
   if(!current.StartsWith(match.Groups[1].Value,StringComparison.Ordinal) || ownership!="1:"+old || current==old)return;
   string temp=Path.Combine(root,".smokey-helper-"+Guid.NewGuid().ToString("N")+".new");
   string backups=Path.Combine(root,"mod-update-backups");Codec.GuardDirectory(backups);
   string backup=Path.Combine(backups,"previous-smokey-updater-"+Guid.NewGuid().ToString("N")+".exe");
   try {
    File.Copy(running,temp,false);if(Codec.HashFile(temp)!=current || Codec.HashFile(primary)!=old)return;
    try {File.Replace(temp,primary,backup,true);}catch(IOException){return;}catch(UnauthorizedAccessException){return;}
    if(Codec.HashFile(primary)==current)Codec.WriteAtomic(marker,System.Text.Encoding.ASCII.GetBytes("1:"+current));
   } finally {if(File.Exists(temp))File.Delete(temp);}
  }
  internal static int CheckOnly(string root,ITransport transport) {
   string receipt=Path.Combine(root,"manual-update.json"),statePath=Path.Combine(root,"update-state.json"),channel=Path.Combine(root,"update-channel.json"),game=ManualInstall.GamePath(root);
   if(!File.Exists(receipt)||!File.Exists(statePath)||!File.Exists(game))return 3;
   var installed=Codec.Read<ManualReceipt>(File.ReadAllBytes(receipt));var state=Codec.Read<State>(File.ReadAllBytes(statePath));
   if(installed==null||state==null||installed.Sequence<1||state.Current!=installed.Sequence||state.Highest<state.Current)return 2;
   string folder=Path.Combine(root,"versions",installed.Sequence.ToString("D12"));string manifest=Path.Combine(folder,"release.json");
   if(!File.Exists(manifest))return 2;
   var previous=Codec.Verify(File.ReadAllBytes(manifest),Trust.PublicKey);var oldExe=previous.Files.Single(f=>f.Name=="skate2-pc.exe");
   if(previous.Sequence!=installed.Sequence||installed.Hash!=oldExe.Hash)return 2;
   if(Codec.HashFile(game)!=oldExe.Hash) {
    // An unverified local preview can explain a newer game release, but may
    // not advertise a signed mod update as installable.
    string upstream=OfficialVersion.Fetch(transport);
    return OfficialVersion.Parse(previous.GameVersion).CompareTo(OfficialVersion.Parse(upstream))<0 ? 11 : 2;
   }
   Console.WriteLine(installed.Sequence);
   string address=File.Exists(channel)?Codec.Read<Channel>(File.ReadAllBytes(channel)).FeedUrl:Feed;var uri=new Uri(address,UriKind.Absolute);
   if(uri.Scheme!="https" || !String.IsNullOrEmpty(uri.UserInfo)||!String.IsNullOrEmpty(uri.Query)||!String.IsNullOrEmpty(uri.Fragment))return 2;
   var latest=Codec.Verify(transport.Get(new Uri(uri.AbsoluteUri+"?check="+DateTime.UtcNow.Ticks),Codec.MaxManifest),Trust.PublicKey);
   string official=OfficialVersion.Fetch(transport);
   if(OfficialVersion.Parse(latest.GameVersion).CompareTo(OfficialVersion.Parse(official))<0)return 11;
   if(latest.Sequence<=state.Highest || (state.Blocked!=null&&state.Blocked.Contains(latest.Sequence)))return 0;
   if(new Version(latest.GameVersion).CompareTo(new Version(previous.GameVersion))<0)return 2;
   return 10;
  }
 }
 sealed class SmokeyWindow : Form {
  readonly string root;readonly bool preview,resumeGame,fallbackGame;readonly Label message;readonly ProgressBar progress;readonly Button close,uninstall;readonly Label details;string updateText=LatestText();
  bool finished;
  static readonly Color Pink=Color.FromArgb(255,0,216), Ink=Color.FromArgb(16,14,21), Card=Color.FromArgb(29,26,35), Muted=Color.FromArgb(202,193,207);
  internal SmokeyWindow(string directory,bool mock,bool reopen=false,bool fallback=false) {
   root=directory;preview=mock;resumeGame=reopen;fallbackGame=fallback;Text="Smokey's Skate Mods";ClientSize=new Size(510,405);StartPosition=FormStartPosition.CenterScreen;FormBorderStyle=FormBorderStyle.FixedDialog;MaximizeBox=false;BackColor=Ink;ForeColor=Color.White;Font=new Font("Segoe UI",9F);Icon=Icon.ExtractAssociatedIcon(Application.ExecutablePath);
   var logo=new PictureBox{Location=new Point(20,12),Size=new Size(64,64),SizeMode=PictureBoxSizeMode.Zoom,BackColor=Color.Black};
   using(var stream=System.Reflection.Assembly.GetExecutingAssembly().GetManifestResourceStream("smokey.wordmark")){if(stream!=null)using(var original=Image.FromStream(stream))logo.Image=new Bitmap(original);}
   var category=new Label{Text="SKATE 2  /  MOD UPDATER",ForeColor=Pink,Font=new Font("Segoe UI",8F,FontStyle.Bold),Location=new Point(103,29),Size=new Size(245,20)};
   var title=new Label{Text="A little more fun on your board.",ForeColor=Color.White,Font=new Font("Segoe UI",10F,FontStyle.Bold),Location=new Point(22,84),Size=new Size(466,25)};
   message=new Label{Text=mock?"Checking for updates...":"Getting things ready...",ForeColor=Muted,Location=new Point(22,113),Size=new Size(465,37)};
   progress=new ProgressBar{Location=new Point(24,154),Size=new Size(460,7),Style=ProgressBarStyle.Marquee};
   var heading=new LinkLabel{Text="WHAT'S NEW",LinkColor=Pink,ActiveLinkColor=Color.White,Font=new Font("Segoe UI",8F,FontStyle.Bold),Location=new Point(24,176),Size=new Size(105,18)};
   var features=new LinkLabel{Text="FEATURES",LinkColor=Muted,ActiveLinkColor=Color.White,Font=new Font("Segoe UI",8F,FontStyle.Bold),Location=new Point(145,176),Size=new Size(105,18)};
   details=new Label{Location=new Point(22,198),Size=new Size(466,141),BackColor=Card,ForeColor=Color.White,Font=new Font("Segoe UI",9F),Padding=new Padding(12,10,8,5),Text=LatestText()};
   heading.LinkClicked+=delegate{details.Text=updateText;heading.LinkColor=Pink;features.LinkColor=Muted;};
   features.LinkClicked+=delegate{details.Text=FeatureText();features.LinkColor=Pink;heading.LinkColor=Muted;};
   uninstall=Button("Uninstall mod",new Point(22,356),132);uninstall.Enabled=mock;uninstall.Click+=Uninstall;
   close=Button("Close",new Point(388,356),100);close.Enabled=mock;close.Click+=delegate{Close();};
   Controls.AddRange(new Control[]{logo,category,title,message,progress,heading,features,details,uninstall,close});
   FormClosing+=delegate(object sender,FormClosingEventArgs e){if(!finished&&!preview)e.Cancel=true;};
   if(!preview)Shown+=StartUpdate;else {finished=true;progress.Style=ProgressBarStyle.Blocks;progress.Value=48;}
  }
  static Button Button(string text,Point at,int width){var b=new Button{Text=text,Location=at,Size=new Size(width,32),BackColor=Color.FromArgb(45,39,49),ForeColor=Color.White,FlatStyle=FlatStyle.Flat,UseVisualStyleBackColor=false};b.FlatAppearance.BorderColor=Pink;return b;}
  static string LatestText(){return "LATEST UPDATE\n\n- Storm clouds, lightning and thunder\n- Water-filled pools and rain effects\n- Water physics for Skate 3 mode\n- More updates are on the way.";}
  static string FeatureText(){return "EXPLORE THE MOD\n\n- Skate 3 physics and Hall of Meat\n- Rain, puddles and storm weather\n- Slippery water and deep-water falls\n- Controller tuning, clean/sketchy and more\n- Find your settings under Options.";}
  void Status(string text) {if(!IsDisposed&&IsHandleCreated) BeginInvoke((Action)delegate{message.Text=text;});}
  async void StartUpdate(object sender,EventArgs e) {
   bool closeAfterLaunch=false;
   try {
    GameVersionMismatchException mismatch=null;
    try {
    if(fallbackGame) {
     bool ready=false;string checkError=null;
     try {
      ready=await Task.Run(delegate {
       var net=new HttpsTransport();string official=OfficialVersion.Fetch(net);
       var saved=Codec.Read<FallbackReceipt>(File.ReadAllBytes(Path.Combine(root,FallbackInstall.ReceiptName)));
       bool needsRepair=false;try { FallbackInstall.VerifiedOfficial(root); }catch(IOException) { needsRepair=true; }
       if(needsRepair || OfficialVersion.Parse(official).CompareTo(OfficialVersion.Parse(saved.Version))>0) {
        string download=OfficialFallback.Prepare(root,official,net,Status);FallbackInstall.UpdateOfficial(root,download,official);
       }
       string channel=Path.Combine(root,"update-channel.json");string feed=File.Exists(channel)?Codec.Read<Channel>(File.ReadAllBytes(channel)).FeedUrl:SmokeyUpdater.Feed;
       return FallbackMode.CheckForMod(root,net,feed,official);
      });
     } catch(Exception error){checkError=error.Message;}
     if(ready && MessageBox.Show("A compatible Smokey mod update is ready. Install it now?","Smokey's Skate Mods",MessageBoxButtons.YesNo,MessageBoxIcon.Information)==DialogResult.Yes) {
      await Task.Run(delegate {FallbackInstall.IsActiveLauncher(root);var client=new Updater(root,Trust.PublicKey,new HttpsTransport(),Status);UpdateFlow.Run(root,client,new HttpsTransport(),true);FallbackInstall.ClearAfterModInstall(root,client);});
      SmokeyUpdater.StartGame(root);closeAfterLaunch=true;return;
     }
     string fallbackNotice=checkError!=null?"The update check could not finish. The saved official game will open without mods. The updater will check again next launch.":
      ready?"The official game will open without mods. You can install the compatible mod update next launch.":"Smokey's mods are temporarily unavailable for the latest game. The official game will open without mods, and the updater will check again next launch.";
     MessageBox.Show(fallbackNotice,"Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);
     SmokeyUpdater.StartOfficial(root);closeAfterLaunch=true;return;
    }
    bool changed=await Task.Run(delegate {SmokeyUpdater.EnsureChannel(root);var client=new Updater(root,Trust.PublicKey,new HttpsTransport(),Status);return UpdateFlow.Run(root,client,new HttpsTransport(),true);});
    message.Text=changed?"Installed! Opening the game...":"You're up to date.";
    LoadReleaseNotes();
    if(changed || resumeGame) {
     SmokeyUpdater.StartGame(root);closeAfterLaunch=true;
    }
    } catch(GameVersionMismatchException error) { mismatch=error; }
    if(mismatch!=null) {
     await Task.Run(delegate {string modHash=FallbackMode.VerifiedInstalledModHash(root);var net=new HttpsTransport();string official=OfficialFallback.Prepare(root,mismatch.OfficialVersion,net,Status);FallbackInstall.Install(root,official,modHash,mismatch.OfficialVersion,SmokeyUpdater.FallbackLauncherBytes(),Path.GetFileName(Application.ExecutablePath));});
     MessageBox.Show("Smokey's mods are temporarily unavailable for the latest game. The official game will open without mods, and the updater will check for a compatible mod next launch.","Smokey's Skate Mods",MessageBoxButtons.OK,MessageBoxIcon.Information);
     SmokeyUpdater.StartOfficial(root);closeAfterLaunch=true;
    }
   } catch(Exception error){message.Text="Could not finish the update.\n"+error.Message;}
   finally {finished=true;close.Enabled=true;uninstall.Enabled=File.Exists(Path.Combine(root,"manual-update.json"));progress.Visible=false;if(closeAfterLaunch)Close();}
  }
  void LoadReleaseNotes(){
   try {var receipt=Codec.Read<ManualReceipt>(File.ReadAllBytes(Path.Combine(root,"manual-update.json")));string path=Path.Combine(root,"versions",receipt.Sequence.ToString("D12"),"VALIDATION.json");var notes=Codec.Read<Notes>(File.ReadAllBytes(path));if(notes!=null&&notes.ReleaseNotes!=null&&notes.ReleaseNotes.Length>0){updateText="UPDATE "+receipt.Sequence+"\n\n"+String.Join("\n",notes.ReleaseNotes.Take(5).Select(s=>"- "+s).ToArray());details.Text=updateText;}} catch(Exception){}
  }
  void Uninstall(object sender,EventArgs e){
   try {
    string original=SafeRestore.VerifiedOriginal(root);
    if(original==null){MessageBox.Show("No verified pre-mod backup was made by this updater. Choose a known original skate2-pc.exe to restore. Your saves and settings stay in place.",Text,MessageBoxButtons.OK,MessageBoxIcon.Information);using(var picker=new OpenFileDialog{Title="Choose your original Skate 2 executable",Filter="Windows executable (*.exe)|*.exe",CheckFileExists=true}){if(picker.ShowDialog(this)!=DialogResult.OK)return;original=picker.FileName;}}
    if(MessageBox.Show("Restore your previous game executable? Your saves and settings will be kept.",Text,MessageBoxButtons.YesNo,MessageBoxIcon.Question)!=DialogResult.Yes)return;
    SafeRestore.Restore(root,original);message.Text="Game restored. Your saves and settings are still here.";uninstall.Enabled=false;
   }catch(Exception error){MessageBox.Show(error.Message,Text,MessageBoxButtons.OK,MessageBoxIcon.Information);}
  }
 }
}
