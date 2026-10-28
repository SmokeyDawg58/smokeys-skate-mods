using System;
using System.IO;
using System.Linq;
using System.Runtime.Serialization;
namespace SkateShared {
 [DataContract] public sealed class OriginalReceipt {
  [DataMember(Name="file")] public string FileName;
  [DataMember(Name="sha256")] public string Hash;
 }
 public static class SafeRestore {
  static string BackupRoot(string root) { return Path.Combine(root,"mod-update-backups"); }
  static string Receipt(string root) { return Path.Combine(BackupRoot(root),"smokey-original.json"); }
  public static void RememberPreModGame(string root) {
   string target=ManualInstall.GamePath(root);
   if (File.Exists(Receipt(root)) || File.Exists(Path.Combine(root,"manual-update.json")) || File.Exists(Path.Combine(root,"update-state.json"))) return;
   ManualInstall.EnsureAvailable(root);
   if (!File.Exists(target)) throw new IOException("Put the updater beside your original skate2-pc.exe first.");
   string folder=BackupRoot(root);Codec.GuardDirectory(folder);string name="before-smokey-"+Guid.NewGuid().ToString("N")+".exe";
   string backup=Path.Combine(folder,name);File.Copy(target,backup,false);string hash=Codec.HashFile(target);
   if (Codec.HashFile(backup)!=hash) throw new IOException("The original game backup could not be verified.");
   Codec.WriteAtomic(Receipt(root),Codec.Json(new OriginalReceipt { FileName=name,Hash=hash }));
  }
  public static string VerifiedOriginal(string root) {
   string receipt=Receipt(root);if (!File.Exists(receipt)) return null;
   var item=Codec.Read<OriginalReceipt>(File.ReadAllBytes(receipt));
   if (item==null || !Codec.SafeName(item.FileName) || item.Hash==null || item.Hash.Length!=64) throw new IOException("The original-game backup record is invalid.");
   string path=Path.Combine(BackupRoot(root),item.FileName);Codec.GuardDirectory(BackupRoot(root));
   if (!File.Exists(path) || (File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0 || Codec.HashFile(path)!=item.Hash) throw new IOException("The saved original game backup could not be verified.");
   return path;
  }
  public static void Restore(string root,string original) {
   ManualInstall.EnsureAvailable(root);string target=ManualInstall.GamePath(root);
   if (!File.Exists(target) || !File.Exists(original) || (File.GetAttributes(original)&FileAttributes.ReparsePoint)!=0) throw new IOException("A valid game executable and original backup are required.");
   string receiptPath=Path.Combine(root,"manual-update.json");
   if (!File.Exists(receiptPath)) throw new IOException("No Smokey mod installation was recorded. Your game was not changed.");
   var installed=Codec.Read<ManualReceipt>(File.ReadAllBytes(receiptPath));
   if (installed==null || installed.Hash==null || Codec.HashFile(target)!=installed.Hash) throw new IOException("Your game EXE changed since the mod was installed. It was left untouched.");
   using (var input=File.OpenRead(original)) { if (input.Length<2 || input.ReadByte()!=77 || input.ReadByte()!=90) throw new IOException("Select a valid Windows game executable."); }
   string hash=Codec.HashFile(original);string folder=BackupRoot(root);Codec.GuardDirectory(folder);
   string stage=Path.Combine(root,".smokey-restore-"+Guid.NewGuid().ToString("N")+".new");
   string backup=Path.Combine(folder,"before-uninstall-"+Guid.NewGuid().ToString("N")+".exe");
   try {
    File.Copy(original,stage,false);if (Codec.HashFile(stage)!=hash) throw new IOException("The restored EXE failed its file check.");
    ManualInstall.EnsureAvailable(root);File.Replace(stage,target,backup,true);
    if (Codec.HashFile(target)!=hash) throw new IOException("The restored game could not be verified. A copy of the mod EXE remains in mod-update-backups.");
   } finally {if (File.Exists(stage)) File.Delete(stage);}
  }
 }
}
