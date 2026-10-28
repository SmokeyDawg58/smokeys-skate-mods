using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.Serialization;
using System.Text;
using System.Text.RegularExpressions;

namespace SkateShared
{
    public sealed class GameVersionMismatchException : InvalidOperationException
    {
        public readonly string OfficialVersion;
        public GameVersionMismatchException(string official,string mod)
            : base("The official game is v" + official + ", but the mod is still v" + mod + ".") { OfficialVersion=official; }
    }
    public static class OfficialVersion
    {
        public const string Address = "https://github.com/skate6743/skate2-rust-port/releases/latest/download/latest-release.txt";
        public static Version Parse(string value)
        {
            Version parsed;
            if (value == null || !Regex.IsMatch(value, @"\A\d{1,5}\.\d{1,5}\.\d{1,5}\z") || !Version.TryParse(value, out parsed))
                throw new InvalidDataException("The update version is invalid.");
            return parsed;
        }
        public static string Fetch(ITransport transport)
        {
            string text = Encoding.UTF8.GetString(transport.Get(new Uri(Address), 65536)).Replace("\r", "");
            if (!text.StartsWith("skate2 release 1\n", StringComparison.Ordinal))
                throw new InvalidDataException("The official release check returned an unexpected response.");
            var match = Regex.Match(text, @"(?m)^version (\d{1,5}\.\d{1,5}\.\d{1,5})$");
            if (!match.Success) throw new InvalidDataException("The official release version is missing.");
            Parse(match.Groups[1].Value);
            return match.Groups[1].Value;
        }
        public static void RequireSupported(string official, Release release)
        {
            if (Parse(release.GameVersion).CompareTo(Parse(official)) != 0)
                throw new GameVersionMismatchException(official, release.GameVersion);
        }
    }

    [DataContract] sealed class DemandSettings
    {
        [DataMember(Name="python")] public string Python { get; set; }
        [DataMember(Name="script")] public string Script { get; set; }
        [DataMember(Name="config")] public string Config { get; set; }
    }
    [DataContract] sealed class DemandStatus
    {
        [DataMember(Name="state")] public string State { get; set; }
        [DataMember(Name="detail")] public string Detail { get; set; }
        [DataMember(Name="draft_feed_url")] public string DraftFeed { get; set; }
    }
    public static class DemandBuilder
    {
        public const string SettingsName = "shared-build-owner.json";
        static string Quote(string value)
        {
            var result = new StringBuilder("\""); int slashes = 0;
            foreach (char c in value) {
                if (c == '\\') slashes++;
                else if (c == '"') { result.Append('\\', slashes * 2 + 1); result.Append(c); slashes = 0; }
                else { result.Append('\\', slashes); result.Append(c); slashes = 0; }
            }
            result.Append('\\', slashes * 2); result.Append('"'); return result.ToString();
        }
        static string ExistingPath(string value)
        {
            if (String.IsNullOrWhiteSpace(value) || !Path.IsPathRooted(value) || !File.Exists(value))
                throw new IOException("The local shared builder needs its saved setup repaired.");
            return Path.GetFullPath(value);
        }
        public static string Run(string root, Action<string> progress, bool approvePreview)
        {
            string settingsPath = Path.Combine(root, SettingsName);
            if (!File.Exists(settingsPath)) {
                if (approvePreview) throw new IOException("Only the configured builder PC can publish a tested preview.");
                return null;
            }
            var settings = Codec.Read<DemandSettings>(File.ReadAllBytes(settingsPath));
            string python = ExistingPath(settings.Python), script = ExistingPath(settings.Script), config = ExistingPath(settings.Config);
            if (!script.EndsWith(".py", StringComparison.OrdinalIgnoreCase) || !config.EndsWith(".json", StringComparison.OrdinalIgnoreCase))
                throw new IOException("The local builder setup has invalid file types.");
            progress("Preparing and testing the shared mod on this builder PC. It runs only for this update.");
            var info = new ProcessStartInfo(python) { UseShellExecute = false, CreateNoWindow = true,
                WorkingDirectory = Path.GetDirectoryName(script), RedirectStandardOutput = true, RedirectStandardError = true,
                Arguments = Quote(script) + " --config " + Quote(config) + (approvePreview ? " --approve-preview" : "") };
            using (var process = new Process { StartInfo = info }) {
                process.OutputDataReceived += delegate(object sender, DataReceivedEventArgs e) {
                    if (!String.IsNullOrWhiteSpace(e.Data)) progress(e.Data);
                };
                // Detailed failures stay in the builder's private logs, never in the friends package.
                process.ErrorDataReceived += delegate(object sender, DataReceivedEventArgs e) { };
                process.Start(); process.BeginOutputReadLine(); process.BeginErrorReadLine();
                process.WaitForExit();
                if (process.ExitCode != 0) throw new IOException("The compatible mod build needs a repair. Your installed game was kept. See the builder status for details.");
            }
            string statusPath = Path.Combine(Path.GetDirectoryName(config), "status.json");
            if (!File.Exists(statusPath)) throw new IOException("The shared builder did not return a result.");
            var state = Codec.Read<DemandStatus>(File.ReadAllBytes(statusPath));
            if (!String.IsNullOrWhiteSpace(state.Detail)) progress(state.Detail);
            if (!String.IsNullOrWhiteSpace(state.DraftFeed)) return state.DraftFeed;
            return null;
        }
    }

    public static class UpdateFlow
    {
        // A signed download is not evidence that it supports the current official game.
        // Check the upstream version before promoting any EXE at the original icon path.
        public static bool Run(string root, Updater client, ITransport network, bool checkDownloads, bool approvePreview = false)
        {
            ManualInstall.EnsureAvailable(root);
            if (!File.Exists(ManualInstall.GamePath(root))) throw new IOException("Put this updater beside your existing skate2-pc.exe first.");
            SafeRestore.RememberPreModGame(root);
            client.RecoverInterruptedLaunch();
            if (checkDownloads) {
                client.Report("Checking the current official game version...");
                string official = OfficialVersion.Fetch(network);
                client.Report("Current official game: v" + official + ".");
                var channel = Codec.Read<Channel>(File.ReadAllBytes(Path.Combine(root, "update-channel.json")));
                if (String.IsNullOrWhiteSpace(channel.FeedUrl)) throw new IOException("The mod download address is missing.");
                string draft = null;
                IOException buildError = null;
                try { draft = DemandBuilder.Run(root, client.Report, approvePreview); }
                catch (IOException error) {
                    if (approvePreview) throw;
                    // A failed owner-side build must not hide an upstream version
                    // mismatch: the caller can still offer the official game.
                    buildError = error;
                }
                client.Check(String.IsNullOrWhiteSpace(draft) ? channel.FeedUrl : draft);
                // A shared build can take a while. Do not promote a now-stale
                // game if another official release arrived during that build.
                official = OfficialVersion.Fetch(network);
                OfficialVersion.RequireSupported(official, client.Installed(client.State.Current));
                if (buildError != null) throw buildError;
                if (!String.IsNullOrWhiteSpace(draft)) client.Report("Installing the checked local-test build through the updater. Shared delivery is awaiting the gameplay check.");
            }
            bool changed = ManualInstall.InstallCurrent(root, client);
            string version = client.Installed(client.State.Current).GameVersion;
            client.Report(changed ? "Installed mod v" + version + ". Your existing game icon is ready." : "Installed mod v" + version + " matches the current checked download.");
            return changed;
        }
    }
}
