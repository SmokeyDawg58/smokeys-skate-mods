# Services (sign-in, Plasma, web)

Everything online that is not the game session itself: signing in, the Plasma services behind
the menus, and the HTTP web services. Addresses are `sk82_na_m.xex` (Xbox 360); Arcadia covers
the PS3 equivalents.

## 1. The `Sk8::Net` façade

Game code never calls the Plasma SDK directly. It posts **input events** to `Sk8::Net::Core`
(`PostEvent` 0x82781a50) and receives **output events** through listeners; transactions and HTTP
requests have their own entry points (`StartTransaction`, `StartHTTPRequest`).

- `Sk8::Net::InputEventType`: LoginAccount, PrepareForCreateAccount, CreateAccount, AcceptTOS,
  ConfirmAccount, ConfirmMessage, RepromptDisplay, CancelSignIn, SignInBackDisplay, Connect,
  Disconnect, EnumGames, JoinGame, JoinGame_UserId, LeaveGame, SetVoiceEnable, StartGame,
  GetFriendList, GetSkatingWith, Stats_GetSkillForOwners, ShowPlayerReview, ShowFriendsRequest,
  ShowGamerCard, ShowMessageCompose, ShowGameInvite, PlayNow, CancelPlayNow,
  GetRecentSkatersList, JoinGame_GameInvite, LookupGame, LookupProgression
  (handled by `Sk8::Net::EventSystem::HandleEventImpl` 0x827835a0 and the platform part).
- `Sk8::Net::OutputEventType`: Display{Login, TOS, CreateAccount, ParentalEmail,
  ConfirmMessage}, Connected, Disconnected, Error, LobbyList, GameList, GameJoined,
  GameJoinFailed, GameQueued, GameQueuePositionUpdate, GameLeft, GameQueuedLeft, PlayerJoin,
  PlayerLeave, GameStart, FriendList, RecentPlayerList, PlayersSkatingWith, presence/friend/recent
  player updates, transaction completed/cancelled, PlayNowResult, PlayNowJoinGame,
  UpdatePlayersList, RecentSkatersListUpdated, GameDetail, AchievementGranted/Removed,
  ProgressionLevelChange, ProgressionLookup.

`Sk8::Net::SystemHub` owns the systems: Plasma (`PlasmaSystem`), messages (`MessageSystem`),
events (`EventSystem`), HTTP (`HTTPRequestSystem`), presence, ads (`Massive`), config.

## 2. Signing in

`Sk8::Flow::StateOnlineSignIn` (`ConnectToPlasma` 0x8275df08, `ConnectingToPlasma` 0x8275e730):
"signing in" popup (cancellable or not), then `IE_Connect` with the controller's user index and
the language. The SDK's connection state machine (`Fesl::ConnectionFSM`, states in
`Fesl::ConnectionStates`) then runs, roughly:

1. `Preconnect` and `ConnectToFesl`: resolve the FESL host (`FeslNamingImpl`; built from the
   client string and `.fesl.ea.com` — Arcadia serves `skate2-ps3.fesl.ea.com` for PS3; dev/alpha
   environments `fesl-dev`, `fesl-alpha`; host and port overrides exist), connect over SSL,
   `fsys` Hello (`ConnectServiceImpl::Hello`), which also tells the client the Theater address
   (`theaterIp`, `theaterPort`) and the domain partition.
2. `Xb360ShowSigninUI` / `Xb360DirtyConnect` / `Xb360Login`: Xbox LIVE profile and the account
   login (`AccountServiceImpl::NuXBL360Login`; PS3 uses `NuPS3Login` with the PSN ticket).
   No account → `PromptCreateUser` (`NuXBL360AddAccount`); terms of service → `PromptTOS`.
3. `ConnectToSubsystems`, `MakeAuxiliaryConnections`: Theater, presence and the rest; then
   `OE_Connected`.

Results surface as Skate errors: underage/restricted, cancelled, connection failed, banned or
forced off (`OutputEvent_Disconnected::DisconnectError`: disconnected from EA server, from the
service provider, by user, banned, forced logoff).

## 3. Plasma services used

| Service (`Fesl::*Impl`) | Skate uses it for |
| --- | --- |
| Connect (`fsys`) | Hello, ping sites (`GetPingSites`, used by PlayNow's ping preference), ping, goodbye |
| Account (`acct`) | login, account creation, terms of service, entitlements (DLC), account info |
| PlayNow (`pnow`) | matchmaking ([matchmaking.md](matchmaking.md)) |
| GameManager / Theater | games ([sessions-and-transport.md](sessions-and-transport.md)) |
| Ranking (`rank`) | skills, ranked stats, leaderboards, spot best scores (`Sk8::Net::TransactionType`: GetSkillForOwners, overall and challenge stats for users / top N, best score, upload stats, achievement stats, SetStat) |
| Record | per-user records (recent skaters: `GetRecentSkaterUserIds` / `UpdateRecentSkaterUserIds`, up to 50) |
| Association (`asso`) | friends and recent players |
| Presence (`pres`) | `Sk8::Net::NetPresence`: skating in, main menu, pause menu, skate shop, online free skating, ranked/unranked trick challenge or race event, create-a-character, inactive; with the district (`PresenceDistrict`) |
| Achievement | awards (46 offline achievement ids registered at init) |
| Telemetry | event logging with a telemetry token |

## 4. Remote configuration

`HTTPRequest_GetConfig` 0x827a1bd0: `GET /Skate2/Config/<domain partition>/RemoteConfig.xml` on
the web base URL. The parsed `Sk8::Xml::Config` carries `GamePlaySettings` (send rate, scale,
slow-down and speed-up factors; [lockstep.md](lockstep.md) §8), `PlayNowSettings` (pool timeout and
thresholds; [matchmaking.md](matchmaking.md) §2), `ParentalControlSettings` and the HTTP request
timeout (default 10 s, `HTTPRequestSystem::OnConfigLoaded` 0x82788e20). Systems that care
observe the config (`OnConfigLoaded`); built-in defaults apply until it arrives.

## 5. Web services

Base URL = FESL's `customUrl` (`Sk8::Net::GetBaseUrl` 0x82784498 →
`Connection::GetCustomUrl`). Requests are HTTP GET (or multipart POST for uploads) to ASP.NET
endpoints built by `GetWebServiceBaseUrl` 0x827a0bb8:

| `ServiceType` | Path |
| --- | --- |
| SkateReel | `/Game/SkateReel.asmx/<function>` |
| SkateProfile | `/Game/SkateProfile.asmx/<function>` |
| SkateDLC | `/Game/SkateDLC.asmx/<function>` |
| SkateCreateASpot | `/Game/SkateCAS.asmx/<function>` |
| SkateMessage | `/Game/SkateM…asmx/<function>` |

Query strings start `?platformId=1` on Xbox 360 (`HTTPRequest_WebServiceGetRequest` 0x827a2c58)
and add `&name=value` pairs (`AddParam` 0x8279f9f0). Content downloads:
`GetFileContent?platformId=…&fileId=…[&type=.flv]` (Skate.Reel) and
`GetSpotContent?platformId=…&spotId=…` (Create-a-Spot).

Requests (`Sk8::Net::HTTPRequestType`):

- **Skate.Reel** (videos and photos): GetContent, GetTopN, GetTopNSummary, GetThumbnail,
  GetPhoto, GetCategoriesAndTags, Vote, IsVoted, Remove, GetPhotoCount, GetTotalVideoSize;
  uploads (`StateOnlineUpload`, `NetUploadable`, 1 KB chunks, up to 4 files and 30 params).
- **Skate profile**: GetProfile, GetOverallRanks, UpdateBadgesAndRanks,
  UpdateAndGetNewBadgeAndRank (after every online challenge), GetBestRank, GetLatestCASDBUrl,
  GetCASPackUrls, GetCustomLogo, GetLogos, ValidatePromoCode.
- **Create-a-Spot**: GetTopN, GetSpotHistory, GetSpotsByIds, GetTaggedSpots, ReportScore, Vote,
  IsVoted, categories and tags, spot file get/upload (`StateOnlineCreateASpotDataFlow`).

`Sk8::Net::Massive` is the in-game advertising client (Massive Ad Client 3), independent of the
rest.

## 6. For the port

None of this is needed to play online together; it is the community layer around it. A port can
start with sign-in (any identity), the remote config values, profile ranks for ranked play, and
add Skate.Reel / Create-a-Spot sharing later. Arcadia already answers most of these (FESL,
static config XML, the profile route).
