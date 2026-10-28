using System;
using System.IO;
using System.Linq;
using System.Runtime.Serialization;

namespace SkateShared
{
    [DataContract] public sealed class ManualReceipt
    {
        [DataMember(Name="sequence")] public long Sequence;
        [DataMember(Name="game_version")] public string GameVersion;
        [DataMember(Name="exe_sha256")] public string Hash;
        [DataMember(Name="installed_utc")] public string InstalledUtc;
        [DataMember(Name="backup_file")] public string BackupFile;
        [DataMember(Name="play_executable")] public string PlayExecutable;
    }

    // Only the signed game EXE is promoted. User settings, saves and ROM are not update inputs.
    public static class ManualInstall
    {
        public static string GamePath(string root) { return Path.Combine(root, "skate2-pc.exe"); }

        public static void EnsureAvailable(string root)
        {
            Codec.GuardDirectory(root);
            string target = GamePath(root);
            if (!File.Exists(target)) return;
            if ((File.GetAttributes(target) & FileAttributes.ReparsePoint) != 0)
                throw new IOException("The game EXE cannot be a linked file.");
            // Windows refuses this when the game is running or another program is writing it.
            try { using (File.Open(target, FileMode.Open, FileAccess.ReadWrite, FileShare.None)) { } }
            catch (IOException error) {
                throw new IOException("Close Skate 2, then open the updater again. The game EXE is currently in use.", error);
            }
        }

        public static bool InstallCurrent(string root, Updater updater)
        {
            root = Path.GetFullPath(root);
            EnsureAvailable(root);
            Release release = updater.Installed(updater.State.Current);
            Blob executable = release.Files.Single(file => file.Name == "skate2-pc.exe");
            string target = GamePath(root);
            string source = Path.Combine(updater.VersionDirectory(release.Sequence), executable.Name);
            bool same = File.Exists(target) && new FileInfo(target).Length == executable.Size && Codec.HashFile(target) == executable.Hash;
            string backup = "";
            if (!same) {
                string stage = Path.Combine(root, ".manual-game-" + Guid.NewGuid().ToString("N") + ".new");
                try {
                    using (var input = File.OpenRead(source))
                    using (var output = new FileStream(stage, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                        input.CopyTo(output);
                        output.Flush(true);
                    }
                    Codec.VerifyFile(stage, executable);
                    EnsureAvailable(root);
                    if (File.Exists(target)) {
                        string backupRoot = Path.Combine(root, "mod-update-backups");
                        Codec.GuardDirectory(backupRoot);
                        backup = Path.Combine(backupRoot, "skate2-pc-" + DateTime.UtcNow.ToString("yyyyMMdd-HHmmss") + "-" + Guid.NewGuid().ToString("N") + ".exe");
                        File.Replace(stage, target, backup, true);
                    } else File.Move(stage, target);
                } finally { if (File.Exists(stage)) File.Delete(stage); }
            }
            Codec.VerifyFile(target, executable);
            // There is no child process to supervise when the player uses the original EXE.
            // Signed previous releases and the replaced root EXE remain available for manual recovery.
            updater.FinishGame(0);
            var receipt = new ManualReceipt { Sequence = release.Sequence, GameVersion = release.GameVersion,
                Hash = executable.Hash, InstalledUtc = DateTime.UtcNow.ToString("o"), BackupFile = backup,
                PlayExecutable = "skate2-pc.exe" };
            string receiptPath = Path.Combine(root, "manual-update.json");
            // A repeat check must not lose the location of the previous EXE backup.
            if (same && File.Exists(receiptPath)) {
                var previous = Codec.Read<ManualReceipt>(File.ReadAllBytes(receiptPath));
                if (previous.Sequence == release.Sequence && previous.Hash == executable.Hash)
                    receipt.BackupFile = previous.BackupFile;
            }
            Codec.WriteAtomic(receiptPath, Codec.Json(receipt));
            return !same;
        }

        public static void VerifyCurrent(string root, Updater updater)
        {
            Release release = updater.Installed(updater.State.Current);
            Codec.VerifyFile(GamePath(root), release.Files.Single(file => file.Name == "skate2-pc.exe"));
        }
    }
}
