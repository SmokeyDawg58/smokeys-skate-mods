// Portable Windows updater. No game code, saves or publisher secrets live here.
using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Linq;
using System.Net;
using System.Runtime.Serialization;
using System.Runtime.Serialization.Json;
using System.Security.Cryptography;
using System.Text;
using System.Text.RegularExpressions;

namespace SkateShared
{
    [DataContract] public sealed class Blob
    {
        [DataMember(Name="name", IsRequired=true)] public string Name;
        [DataMember(Name="size", IsRequired=true)] public long Size;
        [DataMember(Name="sha256", IsRequired=true)] public string Hash;
    }
    [DataContract] public sealed class Release
    {
        [DataMember(Name="format", IsRequired=true)] public int Format;
        [DataMember(Name="channel", IsRequired=true)] public string Channel;
        [DataMember(Name="sequence", IsRequired=true)] public long Sequence;
        [DataMember(Name="game_version", IsRequired=true)] public string GameVersion;
        [DataMember(Name="upstream_commit", IsRequired=true)] public string Commit;
        [DataMember(Name="archive_size", IsRequired=true)] public long ArchiveSize;
        [DataMember(Name="archive_sha256", IsRequired=true)] public string ArchiveHash;
        [DataMember(Name="parts", IsRequired=true)] public Blob[] Parts;
        [DataMember(Name="files", IsRequired=true)] public Blob[] Files;
    }
    [DataContract] public sealed class Envelope
    {
        [DataMember(Name="payload", IsRequired=true)] public string Payload;
        [DataMember(Name="signature", IsRequired=true)] public string Signature;
    }
    [DataContract] public sealed class Channel
    {
        [DataMember(Name="feed_url")] public string FeedUrl;
    }
    [DataContract] public sealed class State
    {
        [DataMember(Name="current")] public long Current;
        [DataMember(Name="previous")] public long Previous;
        [DataMember(Name="highest")] public long Highest;
        [DataMember(Name="pending")] public bool Pending;
        [DataMember(Name="launch_started")] public bool LaunchStarted;
        [DataMember(Name="blocked")] public List<long> Blocked = new List<long>();
    }

    public static class Codec
    {
        public const long MaxArchive = 256L * 1024 * 1024;
        public const long MaxExpanded = 512L * 1024 * 1024;
        public const int MaxManifest = 128 * 1024;
        static readonly byte[] Context = Encoding.UTF8.GetBytes("skate2 shared mod release 1\0");
        static readonly string[] Required = { "skate2-pc.exe", "SOURCE.zip", "LICENSE", "LICENSE-Skate3-engine.txt", "VALIDATION.json" };
        public static byte[] Json<T>(T value)
        {
            using (var stream = new MemoryStream()) {
                new DataContractJsonSerializer(typeof(T)).WriteObject(stream, value);
                return stream.ToArray();
            }
        }
        public static T Read<T>(byte[] bytes)
        {
            using (var stream = new MemoryStream(bytes, false))
                return (T)new DataContractJsonSerializer(typeof(T)).ReadObject(stream);
        }
        public static string HashFile(string path)
        {
            using (var stream = File.OpenRead(path)) using (var sha = SHA256.Create())
                return Hex(sha.ComputeHash(stream));
        }
        public static string HashBytes(byte[] bytes)
        {
            using (var sha = SHA256.Create()) return Hex(sha.ComputeHash(bytes));
        }
        static string Hex(byte[] bytes) { return BitConverter.ToString(bytes).Replace("-", "").ToLowerInvariant(); }
        static byte[] SignedBytes(byte[] bytes) { return Context.Concat(bytes).ToArray(); }
        public static byte[] Sign(byte[] payload, string privateXml)
        {
            using (var rsa = new RSACryptoServiceProvider()) {
                rsa.PersistKeyInCsp = false;
                rsa.FromXmlString(privateXml);
                return Json(new Envelope { Payload = Convert.ToBase64String(payload),
                    Signature = Convert.ToBase64String(rsa.SignData(SignedBytes(payload), CryptoConfig.MapNameToOID("SHA256"))) });
            }
        }
        public static Release Verify(byte[] signed, string publicXml)
        {
            if (signed.Length == 0 || signed.Length > MaxManifest) throw new InvalidDataException("Release notice is too large.");
            var envelope = Read<Envelope>(signed);
            var payload = Convert.FromBase64String(envelope.Payload);
            var signature = Convert.FromBase64String(envelope.Signature);
            using (var rsa = new RSACryptoServiceProvider()) {
                rsa.PersistKeyInCsp = false;
                rsa.FromXmlString(publicXml);
                if (rsa.KeySize < 2048 || !rsa.VerifyData(SignedBytes(payload), CryptoConfig.MapNameToOID("SHA256"), signature))
                    throw new InvalidDataException("Release signature did not match this mod's publisher.");
            }
            var release = Read<Release>(payload);
            Validate(release);
            return release;
        }
        public static bool SafeName(string name)
        {
            if (name == null || !Regex.IsMatch(name, @"\A[A-Za-z0-9][A-Za-z0-9._-]{0,110}\z")) return false;
            string stem = name.Split('.')[0];
            return !Regex.IsMatch(stem, @"\A(CON|PRN|AUX|NUL|COM[0-9]|LPT[0-9])\z", RegexOptions.IgnoreCase);
        }
        static void CheckBlobs(Blob[] blobs, long max, string label)
        {
            if (blobs == null || blobs.Length == 0 || blobs.Length > 64) throw new InvalidDataException("Invalid " + label + " list.");
            var names = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            long total = 0;
            foreach (var blob in blobs) {
                if (blob == null || !SafeName(blob.Name) || !names.Add(blob.Name) || blob.Size <= 0 || blob.Size > max ||
                    blob.Hash == null || !Regex.IsMatch(blob.Hash, @"\A[a-f0-9]{64}\z")) throw new InvalidDataException("Invalid " + label + ".");
                total = checked(total + blob.Size);
            }
            if (total > max) throw new InvalidDataException(label + " exceeds size limit.");
        }
        public static void Validate(Release release)
        {
            if (release == null || release.Format != 1 || release.Channel != "skate2-skate3" || release.Sequence < 1 ||
                release.GameVersion == null || !Regex.IsMatch(release.GameVersion, @"\A[0-9]+\.[0-9]+\.[0-9]+\z") ||
                release.Commit == null || !Regex.IsMatch(release.Commit, @"\A[a-f0-9]{40}\z")) throw new InvalidDataException("Unknown release format.");
            CheckBlobs(release.Parts, MaxArchive, "download part");
            CheckBlobs(release.Files, MaxExpanded, "release file");
            if (release.Parts.Sum(p => p.Size) != release.ArchiveSize || release.ArchiveSize > MaxArchive ||
                release.ArchiveHash == null || !Regex.IsMatch(release.ArchiveHash, @"\A[a-f0-9]{64}\z")) throw new InvalidDataException("Invalid archive.");
            foreach (string name in Required)
                if (!release.Files.Any(f => f.Name == name)) throw new InvalidDataException("Release is missing " + name);
            // The package is deliberately limited to immutable game/source/evidence files.
            // Settings and saves can never be supplied by an update archive.
            if (release.Files.Any(f => !Required.Contains(f.Name))) throw new InvalidDataException("Release contains an unexpected file.");
        }
        public static void VerifyFile(string path, Blob blob)
        {
            if (!File.Exists(path) || new FileInfo(path).Length != blob.Size || HashFile(path) != blob.Hash)
                throw new InvalidDataException("File check failed: " + blob.Name);
        }
        public static void WriteAtomic(string path, byte[] bytes)
        {
            GuardDirectory(Path.GetDirectoryName(path));
            string temporary = path + ".new-" + Guid.NewGuid().ToString("N");
            try {
                using (var stream = new FileStream(temporary, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                    stream.Write(bytes, 0, bytes.Length);
                    stream.Flush(true);
                }
                if (File.Exists(path)) File.Replace(temporary, path, path + ".backup", true);
                else File.Move(temporary, path);
            } finally { if (File.Exists(temporary)) File.Delete(temporary); }
        }
        public static void GuardDirectory(string directory)
        {
            var entry = new DirectoryInfo(Path.GetFullPath(directory));
            for (var item = entry; item != null; item = item.Parent)
                if (item.Exists && (item.Attributes & FileAttributes.ReparsePoint) != 0)
                    throw new IOException("The game folder cannot be a linked folder.");
            Directory.CreateDirectory(entry.FullName);
        }
    }

    public interface ITransport { byte[] Get(Uri uri, long limit); void Download(Uri uri, string path, long size, Action<long> progress); }
    public sealed class HttpsTransport : ITransport
    {
        readonly bool loopbackTests;
        public HttpsTransport(bool allowLoopbackTests = false) { loopbackTests = allowLoopbackTests; }
        public void ValidateUri(Uri uri)
        {
            if (uri == null || !uri.IsAbsoluteUri || !String.IsNullOrEmpty(uri.UserInfo) ||
                (uri.Scheme != "https" && !(loopbackTests && uri.Scheme == "http" && uri.IsLoopback)))
                throw new InvalidDataException("The download address must use HTTPS.");
        }
        HttpWebResponse Open(Uri uri)
        {
            ServicePointManager.SecurityProtocol = SecurityProtocolType.Tls12;
            for (int redirects = 0; redirects <= 4; redirects++) {
                ValidateUri(uri);
                var request = (HttpWebRequest)WebRequest.Create(uri);
                request.Timeout = 8000;
                request.ReadWriteTimeout = 10000;
                request.UserAgent = "Skate2SharedMod/1";
                request.AllowAutoRedirect = false;
                request.AutomaticDecompression = DecompressionMethods.None;
                request.CachePolicy = new System.Net.Cache.RequestCachePolicy(System.Net.Cache.RequestCacheLevel.NoCacheNoStore);
                var response = (HttpWebResponse)request.GetResponse();
                int status = (int)response.StatusCode;
                if (status >= 300 && status < 400) {
                    string location = response.Headers["Location"];
                    response.Dispose();
                    if (String.IsNullOrEmpty(location)) throw new IOException("Download redirect has no destination.");
                    uri = new Uri(uri, location);
                    continue;
                }
                if (status != 200) { response.Dispose(); throw new IOException("Download server returned " + status); }
                return response;
            }
            throw new IOException("Too many download redirects.");
        }
        void Copy(Uri uri, Stream target, long limit, Action<long> progress)
        {
            using (var response = Open(uri)) using (var source = response.GetResponseStream()) {
                if (response.ContentLength > limit) throw new InvalidDataException("Download is larger than expected.");
                var timer = Stopwatch.StartNew();
                byte[] buffer = new byte[128 * 1024];
                long total = 0;
                int count;
                while ((count = source.Read(buffer, 0, buffer.Length)) != 0) {
                    total = checked(total + count);
                    if (total > limit || timer.Elapsed.TotalMinutes > 5) throw new IOException("Download exceeded its limit.");
                    target.Write(buffer, 0, count);
                    progress(total);
                }
            }
        }
        public byte[] Get(Uri uri, long limit)
        {
            using (var stream = new MemoryStream()) { Copy(uri, stream, limit, delegate(long n) {}); return stream.ToArray(); }
        }
        public void Download(Uri uri, string path, long size, Action<long> progress)
        {
            using (var stream = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                Copy(uri, stream, size, progress);
                if (stream.Length != size) throw new InvalidDataException("Download was incomplete.");
                stream.Flush(true);
            }
        }
    }

    public sealed class Updater
    {
        readonly string root, publicKey;
        readonly ITransport transport;
        readonly Action<string> status;
        public State State;
        public Updater(string directory, string key, ITransport network, Action<string> progress)
        {
            root = Path.GetFullPath(directory).TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
            if (root.Length < 4) throw new IOException("Extract the game into its own folder first.");
            publicKey = key; transport = network; status = progress;
            Codec.GuardDirectory(root);
            Codec.GuardDirectory(Path.Combine(root, "versions"));
            Codec.GuardDirectory(Path.Combine(root, "profile"));
            string stateFile = Path.Combine(root, "update-state.json");
            if (File.Exists(stateFile)) State = Codec.Read<State>(File.ReadAllBytes(stateFile));
            else State = new State { Current = 0, Highest = 0 };
            if (State.Blocked == null) State.Blocked = new List<long>();
            if (State.Current < 0 || State.Previous < 0 || State.Highest < State.Current) throw new InvalidDataException("Update state is damaged.");
        }
        public string VersionDirectory(long sequence) { return Path.Combine(root, "versions", sequence.ToString("D12")); }
        void Save() { Codec.WriteAtomic(Path.Combine(root, "update-state.json"), Codec.Json(State)); }
        public Release Installed(long sequence)
        {
            string folder = VersionDirectory(sequence);
            Codec.GuardDirectory(folder);
            var release = Codec.Verify(File.ReadAllBytes(Path.Combine(folder, "release.json")), publicKey);
            if (release.Sequence != sequence) throw new InvalidDataException("Installed release has the wrong number.");
            foreach (var file in release.Files) Codec.VerifyFile(Path.Combine(folder, file.Name), file);
            return release;
        }
        public void RecoverInterruptedLaunch()
        {
            if (State.Pending && State.LaunchStarted && State.Previous > 0) {
                Installed(State.Previous);
                State.Blocked.Add(State.Current);
                State.Current = State.Previous;
                State.Previous = 0;
                State.Pending = false;
                State.LaunchStarted = false;
                Save();
                status("The last update did not finish starting. Restored the previous game.");
            }
            if (State.Current > 0) Installed(State.Current);
        }
        public void Report(string message) { status(message); }
        public bool Check(string feedUrl)
        {
            if (String.IsNullOrWhiteSpace(feedUrl)) { status("Shared downloads are not connected yet. Starting the installed game."); return false; }
            var feed = new Uri(feedUrl, UriKind.Absolute);
            if (!String.IsNullOrEmpty(feed.Query) || !String.IsNullOrEmpty(feed.Fragment)) throw new InvalidDataException("Invalid update feed address.");
            status("Checking for a tested mod update...");
            var signed = transport.Get(new Uri(feed.AbsoluteUri + "?check=" + DateTime.UtcNow.Ticks), Codec.MaxManifest);
            var release = Codec.Verify(signed, publicKey);
            if (release.Sequence <= State.Highest || State.Blocked.Contains(release.Sequence)) { status("No newer published mod build was found."); return false; }
            if (State.Current > 0 && new Version(release.GameVersion).CompareTo(new Version(Installed(State.Current).GameVersion)) < 0)
                throw new InvalidDataException("The shared download would downgrade the installed game version.");
            string final = VersionDirectory(release.Sequence);
            string stage = Path.Combine(root, ".update-" + Guid.NewGuid().ToString("N"));
            Codec.GuardDirectory(stage);
            string package = Path.Combine(stage, "download.zip");
            string unpack = Path.Combine(stage, "game");
            Codec.GuardDirectory(unpack);
            try {
                long complete = 0;
                using (var combined = new FileStream(package, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                    foreach (var part in release.Parts) {
                        string local = Path.Combine(stage, part.Name);
                        transport.Download(new Uri(feed, part.Name), local, part.Size, delegate(long n) {
                            status("Downloading mod update: " + ((complete + n) * 100 / release.ArchiveSize) + "%");
                        });
                        Codec.VerifyFile(local, part);
                        using (var input = File.OpenRead(local)) input.CopyTo(combined);
                        complete += part.Size;
                        File.Delete(local);
                    }
                    combined.Flush(true);
                }
                if (new FileInfo(package).Length != release.ArchiveSize || Codec.HashFile(package) != release.ArchiveHash)
                    throw new InvalidDataException("The complete update failed its file check.");
                status("Checking the downloaded game...");
                Extract(package, unpack, release);
                File.WriteAllBytes(Path.Combine(unpack, "release.json"), signed);
                if (Directory.Exists(final)) {
                    // A prior interrupted installation can be reused only if all signed bytes match.
                    Installed(release.Sequence);
                    if (!File.ReadAllBytes(Path.Combine(final, "release.json")).SequenceEqual(signed))
                        throw new InvalidDataException("A different release already uses that number.");
                } else Directory.Move(unpack, final);
                State.Previous = State.Pending ? State.Previous : State.Current;
                State.Current = release.Sequence;
                State.Highest = release.Sequence;
                State.Pending = true;
                State.LaunchStarted = false;
                Save();
                status("Update checked. Starting the new mod.");
                return true;
            } finally { DeleteOwnedStage(stage); }
        }
        public static void Extract(string package, string destination, Release release)
        {
            Codec.GuardDirectory(destination);
            var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
            using (var archive = ZipFile.OpenRead(package)) {
                if (archive.Entries.Count != release.Files.Length) throw new InvalidDataException("Unexpected archive contents.");
                foreach (var entry in archive.Entries) {
                    var file = release.Files.FirstOrDefault(f => f.Name == entry.FullName);
                    int kind = (entry.ExternalAttributes >> 16) & 0xF000;
                    if (file == null || !Codec.SafeName(entry.FullName) || !seen.Add(entry.FullName) ||
                        entry.Length != file.Size || (kind != 0 && kind != 0x8000)) throw new InvalidDataException("Unsafe or unexpected archive entry.");
                    string path = Path.Combine(destination, file.Name);
                    using (var source = entry.Open()) using (var output = new FileStream(path, FileMode.CreateNew, FileAccess.Write, FileShare.None)) {
                        byte[] buffer = new byte[128 * 1024];
                        long total = 0; int count;
                        while ((count = source.Read(buffer, 0, buffer.Length)) != 0) {
                            total += count;
                            if (total > file.Size) throw new InvalidDataException("Expanded file exceeds its declared size.");
                            output.Write(buffer, 0, count);
                        }
                        output.Flush(true);
                    }
                    Codec.VerifyFile(path, file);
                }
            }
        }
        void DeleteOwnedStage(string stage)
        {
            // This path is constructed here from a fresh GUID, never from the feed or a ZIP entry.
            string absolute = Path.GetFullPath(stage);
            if (Path.GetDirectoryName(absolute) != root || !Path.GetFileName(absolute).StartsWith(".update-", StringComparison.Ordinal))
                throw new IOException("Refusing to remove a folder outside the update staging area.");
            try {
                Codec.GuardDirectory(absolute);
                foreach (string folder in Directory.GetDirectories(absolute, "*", SearchOption.AllDirectories)) Codec.GuardDirectory(folder);
                Directory.Delete(absolute, true);
            } catch (IOException) { status("An interrupted download will be left for inspection."); }
        }
        public ProcessStartInfo GameStart()
        {
            Installed(State.Current);
            if (State.Pending) { State.LaunchStarted = true; Save(); }
            string profile = Path.Combine(root, "profile");
            Codec.GuardDirectory(profile);
            var info = new ProcessStartInfo(Path.Combine(VersionDirectory(State.Current), "skate2-pc.exe")) {
                WorkingDirectory = profile, UseShellExecute = false
            };
            info.EnvironmentVariables["SKATE2_SAVE_ROOT"] = profile;
            info.EnvironmentVariables.Remove("SKATE2_GAME_ROOT");
            info.EnvironmentVariables["SKATE2_TEST_LOG_ROOT"] = Path.Combine(profile, "test_logs");
            return info;
        }
        public bool FinishGame(int exitCode)
        {
            if (!State.Pending) return false;
            if (exitCode != 0 && State.Previous > 0) {
                Installed(State.Previous);
                State.Blocked.Add(State.Current);
                State.Current = State.Previous;
                State.Previous = 0;
                State.Pending = false;
                State.LaunchStarted = false;
                Save();
                return true;
            }
            State.Pending = false;
            State.LaunchStarted = false;
            Save();
            return false;
        }
        public void Rollback()
        {
            if (State.Previous <= 0) throw new InvalidOperationException("No previous game is installed yet.");
            Installed(State.Previous);
            State.Blocked.Add(State.Current);
            State.Current = State.Previous;
            State.Previous = 0;
            State.Pending = false;
            State.LaunchStarted = false;
            Save();
        }
    }
}
