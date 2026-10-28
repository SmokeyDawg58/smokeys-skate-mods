// Transactional, opt-in placement of a launcher for the mod-free fallback.
using System;
using System.IO;
using System.Runtime.Serialization;

namespace SkateShared {
 [DataContract] public sealed class FallbackReceipt {
  [DataMember(Name="version")] public string Version;
  [DataMember(Name="official_file")] public string OfficialFile;
  [DataMember(Name="official_sha256")] public string OfficialHash;
  [DataMember(Name="launcher_sha256")] public string LauncherHash;
  [DataMember(Name="previous_mod_sha256")] public string PreviousModHash;
  [DataMember(Name="backup_file")] public string BackupFile;
  [DataMember(Name="updater_file")] public string UpdaterFile;
  [DataMember(Name="updater_sha256")] public string UpdaterHash;
  [DataMember(Name="state")] public string State;
 }
 public static class FallbackInstall {
  public const string ReceiptName="smokey-fallback.json";
  static string ReceiptPath(string root) { return Path.Combine(root,ReceiptName); }
  static void CheckReceipt(FallbackReceipt receipt) {
   if(receipt==null || receipt.State!="active" || !Codec.SafeName(receipt.OfficialFile) ||
      !receipt.OfficialFile.StartsWith("smokey-official-",StringComparison.Ordinal) ||
      !SafeUpdaterName(receipt.UpdaterFile) ||
      !RegexHash(receipt.UpdaterHash) || !RegexHash(receipt.OfficialHash) || !RegexHash(receipt.LauncherHash) || !RegexHash(receipt.PreviousModHash))
    throw new InvalidDataException("The temporary game fallback record is invalid.");
   OfficialVersion.Parse(receipt.Version);
  }
  static bool RegexHash(string value) { return value!=null && System.Text.RegularExpressions.Regex.IsMatch(value,@"\A[a-f0-9]{64}\z"); }
  static bool SafeUpdaterName(string value) { return value!=null && System.Text.RegularExpressions.Regex.IsMatch(value,@"\ASmokey's Skate Mods Updater(?:-[a-f0-9]{8})?\.exe\z"); }
  static void CopyOfficial(string root,string source,string destination,string hash) {
   if(File.Exists(destination)) {
    if((File.GetAttributes(destination)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(destination)!=hash)
     throw new IOException("The existing official game cache could not be verified.");
    return;
   }
   string temp=Path.Combine(root,".smokey-stock-"+Guid.NewGuid().ToString("N")+".new");
   try {
    File.Copy(source,temp,false);
    if(Codec.HashFile(temp)!=hash)throw new IOException("The official game copy failed verification.");
    File.Move(temp,destination);
   } finally { if(File.Exists(temp))File.Delete(temp); }
  }
  public static bool Recover(string root) {
   string path=ReceiptPath(root);if(!File.Exists(path))return false;
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(path));
   if(receipt==null)throw new InvalidDataException("The temporary game fallback record is invalid.");
   if(receipt.State=="active") { CheckReceipt(receipt);return true; }
   if(receipt.State!="installing" || !Codec.SafeName(receipt.BackupFile) ||
      !RegexHash(receipt.LauncherHash) || !RegexHash(receipt.PreviousModHash))
    throw new InvalidDataException("The interrupted fallback record is invalid.");
   string game=ManualInstall.GamePath(root),backup=Path.Combine(root,"mod-update-backups",receipt.BackupFile);
   if(!File.Exists(game))throw new IOException("The interrupted fallback needs repair; game files were kept.");
   string hash=Codec.HashFile(game);
   if(hash==receipt.PreviousModHash) { File.Delete(path);return false; }
   if(hash!=receipt.LauncherHash || !File.Exists(backup) || Codec.HashFile(backup)!=receipt.PreviousModHash)
    throw new IOException("The interrupted fallback needs repair; game files were kept.");
   receipt.State="active";CheckReceipt(receipt);
   string official=Path.Combine(root,receipt.OfficialFile);
   if(!File.Exists(official) || Codec.HashFile(official)!=receipt.OfficialHash)
    throw new IOException("The temporary official game could not be verified.");
   Codec.WriteAtomic(path,Codec.Json(receipt));return true;
  }
  public static string VerifiedOfficial(string root) {
   if(!Recover(root))throw new IOException("No temporary game fallback is installed.");
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(ReceiptPath(root)));CheckReceipt(receipt);
   string target=Path.Combine(root,receipt.OfficialFile);
   if(!File.Exists(target) || (File.GetAttributes(target)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(target)!=receipt.OfficialHash)
    throw new IOException("The temporary official game could not be verified.");
   return target;
  }
  public static bool IsActiveLauncher(string root) {
   if(!Recover(root))return false;
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(ReceiptPath(root)));CheckReceipt(receipt);
   string game=ManualInstall.GamePath(root);
   if(!File.Exists(game) || (File.GetAttributes(game)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(game)!=receipt.LauncherHash)
    throw new IOException("The game icon changed since temporary fallback was installed. It was left untouched.");
   return true;
  }
  public static string VerifiedUpdater(string root) {
   if(!Recover(root))throw new IOException("No temporary game fallback is installed.");
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(ReceiptPath(root)));CheckReceipt(receipt);
   string updater=Path.Combine(root,receipt.UpdaterFile);
   if(!File.Exists(updater) || (File.GetAttributes(updater)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(updater)!=receipt.UpdaterHash)
    throw new IOException("The fallback updater could not be verified.");
   return updater;
  }
  public static void UpdateOfficial(string root,string preparedOfficial,string officialVersion) {
   if(!Recover(root))throw new IOException("No temporary game fallback is installed.");
   OfficialVersion.Parse(officialVersion);
   var receipt=Codec.Read<FallbackReceipt>(File.ReadAllBytes(ReceiptPath(root)));CheckReceipt(receipt);
   if(OfficialVersion.Parse(officialVersion).CompareTo(OfficialVersion.Parse(receipt.Version))<0)
    throw new InvalidDataException("An older official game cannot replace the fallback.");
   if(!File.Exists(preparedOfficial) || (File.GetAttributes(preparedOfficial)&FileAttributes.ReparsePoint)!=0)
    throw new IOException("The verified official game is unavailable.");
   string hash=Codec.HashFile(preparedOfficial);
   string name="smokey-official-"+officialVersion+"-"+hash.Substring(0,12)+".exe";
   string destination=Path.Combine(root,name);
   if(File.Exists(destination) && ((File.GetAttributes(destination)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(destination)!=hash)) {
    name="smokey-official-"+officialVersion+"-"+hash.Substring(0,12)+"-"+Guid.NewGuid().ToString("N")+".exe";
    destination=Path.Combine(root,name);
   }
   CopyOfficial(root,preparedOfficial,destination,hash);
   receipt.Version=officialVersion;receipt.OfficialFile=name;receipt.OfficialHash=hash;
   Codec.WriteAtomic(ReceiptPath(root),Codec.Json(receipt));
  }
  public static void ClearAfterModInstall(string root,Updater updater) {
   // Only clear the fallback after a verified signed mod is back at the original icon.
   ManualInstall.VerifyCurrent(root,updater);
   string path=ReceiptPath(root);if(File.Exists(path))File.Delete(path);
  }
  public static void Install(string root,string preparedOfficial,string expectedModHash,string officialVersion,byte[] launcher,string updaterName) {
   root=Path.GetFullPath(root);Codec.GuardDirectory(root);OfficialVersion.Parse(officialVersion);
   if(!RegexHash(expectedModHash) || launcher==null || launcher.Length<2 || launcher[0]!=77 || launcher[1]!=90)
    throw new InvalidDataException("The temporary launcher inputs are invalid.");
   if(!SafeUpdaterName(updaterName))
    throw new InvalidDataException("The updater name is invalid.");
   string updater=Path.Combine(root,updaterName);
   if(!File.Exists(updater) || (File.GetAttributes(updater)&FileAttributes.ReparsePoint)!=0)
    throw new IOException("The updater must be beside the game.");
   string updaterHash=Codec.HashFile(updater);
   if(File.Exists(ReceiptPath(root)))throw new IOException("A temporary game fallback is already recorded.");
   ManualInstall.EnsureAvailable(root);string target=ManualInstall.GamePath(root);
   if(!File.Exists(target) || Codec.HashFile(target)!=expectedModHash)
    throw new IOException("The installed game changed since the mod was verified. It was left untouched.");
   if(!File.Exists(preparedOfficial) || (File.GetAttributes(preparedOfficial)&FileAttributes.ReparsePoint)!=0)
    throw new IOException("The verified official game is unavailable.");
   string officialHash=Codec.HashFile(preparedOfficial);
   string officialName="smokey-official-"+officialVersion+"-"+officialHash.Substring(0,12)+".exe";
   string official=Path.Combine(root,officialName);string temp=Path.Combine(root,".smokey-launch-"+Guid.NewGuid().ToString("N")+".new");
   string backups=Path.Combine(root,"mod-update-backups");Codec.GuardDirectory(backups);
   string backupName="before-fallback-"+Guid.NewGuid().ToString("N")+".exe";
   string backup=Path.Combine(backups,backupName);
   try {
    CopyOfficial(root,preparedOfficial,official,officialHash);
    using(var output=new FileStream(temp,FileMode.CreateNew,FileAccess.Write,FileShare.None)) { output.Write(launcher,0,launcher.Length);output.Flush(true); }
    string launcherHash=Codec.HashFile(temp);
    var receipt=new FallbackReceipt { Version=officialVersion,OfficialFile=officialName,OfficialHash=officialHash,
     LauncherHash=launcherHash,PreviousModHash=expectedModHash,BackupFile=backupName,State="installing",UpdaterFile=updaterName,UpdaterHash=updaterHash };
    Codec.WriteAtomic(ReceiptPath(root),Codec.Json(receipt));
    ManualInstall.EnsureAvailable(root);
    if(Codec.HashFile(target)!=expectedModHash)throw new IOException("The game changed during fallback preparation.");
    File.Replace(temp,target,backup,true);
    if(Codec.HashFile(target)!=launcherHash || Codec.HashFile(backup)!=expectedModHash)throw new IOException("The fallback could not be verified. The previous game remains in mod-update-backups.");
    receipt.State="active";Codec.WriteAtomic(ReceiptPath(root),Codec.Json(receipt));
   } finally { if(File.Exists(temp))File.Delete(temp); }
  }
 }
}
