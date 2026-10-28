//! Retail front-end ActionScript host. Unknown game services fail explicitly.
use crate::{
    apt_movie::Movie,
    apt_vm::{Host, ObjectKind, Value, Vm},
};

pub struct Runtime {
    pub vm: Vm,
    pub bindings: Bindings,
    pub controller: usize,
}

pub struct Bindings {
    pub movie: Movie,
    pub loaded: bool,
    pub trace: Vec<String>,
    random: u32,
    pub category: usize,
    pub selection: usize,
    pub dimmer: bool,
    pub options: Vec<MenuOption>,
    pub options_title: String,
    pub callbacks: Vec<(String, Vec<Value>)>,
    pub popup_title: String,
    pub popup_description: String,
    pub buttons: std::collections::BTreeMap<String, ButtonTexture>,
}
#[derive(serde::Deserialize)]
pub struct ButtonTexture {
    pub width: u32,
    pub height: u32,
    pub rgba: String,
}

#[derive(Clone)]
pub struct MenuOption {
    pub title: String,
    pub kind: String,
    pub value: String,
    pub description: String,
    pub integer: i32,
    pub enabled: bool,
}
impl MenuOption {
    pub fn option(title: &str, description: &str) -> Self {
        Self {
            title: title.into(),
            kind: "option".into(),
            value: String::new(),
            description: description.into(),
            integer: 0,
            enabled: true,
        }
    }
}

impl Host for Bindings {
    fn property_changed(&mut self, vm: &mut Vm, object: usize, key: &str) -> Result<(), String> {
        if key == "_test" && vm.get(object, "_type").text() == "CustomButton" {
            let name = vm.get(object, key).text();
            if let Some(texture) = self.buttons.get(&name) {
                let bounds = self.movie.dimensions(vm, object).map(|v| v as f32);
                self.movie.external_images.insert(
                    object,
                    crate::apt_movie::ExternalImage {
                        texture: texture.rgba.clone(),
                        bounds,
                    },
                );
            } else if name != "undefined" && name != "Unknown" && !name.is_empty() {
                return Err(format!("Missing original controller button {name}"));
            }
        }
        if key == "text" || key == "autoSize" {
            self.movie.text_changed(vm, object)?;
        }
        if self.movie.instances.contains_key(&object) {
            if key == "_width" || key == "_height" {
                let requested = vm.get(object, key).number();
                let b = self.movie.dimensions(vm, object);
                let extent = if key == "_width" {
                    b[2] - b[0]
                } else {
                    b[3] - b[1]
                };
                if extent > 0. && requested.is_finite() {
                    vm.set(
                        object,
                        if key == "_width" {
                            "_xscale"
                        } else {
                            "_yscale"
                        },
                        Value::Number(requested / extent * 100.),
                    )?;
                }
            }
            self.movie.refresh_dimensions(vm, object)?;
            if let Value::Object(parent) = vm.get(object, "_parent") {
                self.movie.refresh_dimensions(vm, parent)?;
            }
        }
        Ok(())
    }
    fn call(
        &mut self,
        vm: &mut Vm,
        object: usize,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        if self.movie.instances.contains_key(&object) && method == "setTextFormat" {
            if let Some(Value::Object(format)) = args.last() {
                if let Value::Number(size) = vm.get(*format, "size") {
                    vm.set(object, "_fontSize", Value::Number(size))?;
                }
                if let Value::Number(color) = vm.get(*format, "color") {
                    vm.set(object, "textColor", Value::Number(color))?;
                }
                self.movie.text_changed(vm, object)?;
            }
            return Ok(Value::Undefined);
        }
        if self.movie.instances.contains_key(&object) && method == "attachMovie" {
            if args.len() < 3 {
                return Err("attachMovie requires symbol, name and depth".into());
            }
            let earlier = std::mem::take(&mut self.movie.pending);
            let id = self.movie.attach(
                vm,
                object,
                &args[0].text(),
                &args[1].text(),
                args[2].number() as i32,
            )?;
            if let Some(Value::Object(initial)) = args.get(3) {
                for (key, value) in vm.objects[*initial].fields.clone() {
                    vm.set(id, key, value)?;
                }
            }
            let result = self.drain(vm);
            self.movie.pending = earlier;
            result?;
            return Ok(Value::Object(id));
        }
        if self.movie.method(vm, object, method, &args)? {
            return Ok(Value::Undefined);
        }
        let kind = match &vm.objects[object].kind {
            ObjectKind::Native(name) => name.as_str(),
            _ if object == vm.global => "global",
            _ => "object",
        };
        if kind == "Game" && method.starts_with("GameSettings_") {
            if method == "GameSettings_GetNumOptions" {
                return Ok(Value::Number(self.options.len() as f64));
            }
            if method == "GameSettings_GetScreenTitle" {
                return Ok(Value::Text(self.options_title.clone()));
            }
            let index = args.first().map_or(0, |v| v.number() as usize);
            let option = self
                .options
                .get(index)
                .ok_or_else(|| format!("Absent setting row {index}"))?;
            return Ok(match method {
                "GameSettings_GetOptionType" => Value::Text(option.kind.clone()),
                "GameSettings_GetOptionTitle" => Value::Text(option.title.clone()),
                "GameSettings_GetIntegerValue" => Value::Number(option.integer as f64),
                "GameSettings_GetStringValue" => Value::Text(option.value.clone()),
                "GameSettings_GetValueDescription" => Value::Text(option.description.clone()),
                _ => return Err(format!("Unimplemented setting service {method}")),
            });
        }
        match (kind, method) {
            ("Popup", "GetTitle") => Ok(Value::Text(self.popup_title.clone())),
            ("Popup", "GetDescription") => Ok(Value::Text(self.popup_description.clone())),
            ("Popup", "IsPopupCancelable") => Ok(Value::Bool(true)),
            ("Game", "OnMenuGetItemEnabled") => Ok(Value::Bool(
                self.options
                    .get(args.last().map_or(0, |v| v.number() as usize))
                    .is_none_or(|o| o.enabled),
            )),
            ("Game", "OnMenuSelect" | "OnMenuHilight") => {
                self.callbacks.push((method.into(), args));
                Ok(Value::Undefined)
            }
            ("coremenu", "GetIsDemo" | "IsDLCEnabled") => Ok(Value::Bool(false)),
            ("coremenu", "GetMaxSubOptions") => Ok(Value::Number(8.)),
            ("coremenu", "GetProgressionData") => {
                let data = vm.object(ObjectKind::Plain);
                for (i, n) in [-1., 0., 0., 0., 0.].into_iter().enumerate() {
                    vm.set(data, i.to_string(), Value::Number(n))?;
                }
                Ok(Value::Object(data))
            }
            ("coremenu", "GetCurrentModeTitle") => Ok(Value::Text(
                "ID_CROSSBAR_OFFLINE_FREESKATE_MODE_TITLE".into(),
            )),
            ("coremenu", "GetNumMenus") => Ok(Value::Number(5.)),
            ("coremenu", "GetCurrentMenuIndex") => Ok(Value::Number(self.category as f64)),
            ("coremenu", "GetCurrentOptionIndex") => Ok(Value::Number(self.selection as f64)),
            ("coremenu", "GetMenuIconName") => Ok(Value::Text(
                ["singleplayer", "multiplayer", "create", "learn", "options"]
                    [args[0].number() as usize % 5]
                    .into(),
            )),
            ("coremenu", "GetMenuName") => Ok(Value::Text(
                [
                    "ID_CROSSBAR_SINGLE_PLAYER",
                    "ID_CROSSBAR_MULTIPLAYER",
                    "ID_CROSSBAR_CREATE",
                    "ID_CROSSBAR_LEARN",
                    "ID_SETTINGS_GAMESETTINGS",
                ][args[0].number() as usize % 5]
                    .into(),
            )),
            ("coremenu", "GetNumOptions") => Ok(Value::Number(1.)),
            ("coremenu", "GetOptionName") => Ok(Value::Text(
                [
                    "#Challenge Map",
                    "#Session Browser",
                    "ID_CROSSBAR_MY_CAREER_TEAM",
                    "#Trick Guide",
                    "ID_SETTINGS_GAMESETTINGS",
                ][self.category]
                    .into(),
            )),
            ("coremenu", "GetOptionIconName") => Ok(Value::Text(
                ["map", "contacts", "mycareerteam", "trickguide", "settings"][self.category].into(),
            )),
            ("coremenu", "GetOptionHelperDescription") => Ok(Value::Text(
                [
                    "ID_CROSSBAR_CHALLENGE_MAP_HELPER_TEXT",
                    "#Find a Rust engine session",
                    "ID_CROSSBAR_MY_CAREER_TEAM_HELPER_TEXT",
                    "ID_CROSSBAR_TRICK_GUIDE_HELPER_TEXT",
                    "ID_CROSSBAR_GAME_SETTINGS_HELPER_TEXT",
                ][self.category]
                    .into(),
            )),
            ("coremenu", "IsItemEnabled") => Ok(Value::Bool(true)),
            ("Object", "registerClass") => {
                let symbol = args.first().ok_or("registerClass requires symbol")?.text();
                let Value::Object(class) =
                    args.get(1).ok_or("registerClass requires constructor")?
                else {
                    return Err("Invalid registered constructor".into());
                };
                for exports in self.movie.exports.values() {
                    if let Some(&character) = exports.get(&symbol) {
                        self.movie.classes.insert(character, *class);
                    }
                }
                Ok(Value::Bool(true))
            }
            ("global", "ASSetPropFlags") => Ok(Value::Undefined),
            ("global", "trace") => {
                if self.trace.len() < 512 {
                    self.trace
                        .push(args.first().map_or(String::new(), Value::text));
                }
                Ok(Value::Undefined)
            }
            ("ScreenManager", "OnLoaded") => {
                self.loaded = true;
                Ok(Value::Undefined)
            }
            ("ScreenManager", "IntroComplete" | "OutroComplete") => Ok(Value::Undefined),
            ("ScreenManager", "EnableDimmer") => {
                self.dimmer = args.first().is_some_and(Value::truth);
                Ok(Value::Undefined)
            }
            ("LetterBox", "EnableLetterBox" | "EnableLetterBoxInfo") => Ok(Value::Undefined),
            ("Audio", "PlaySound") => Ok(Value::Undefined),
            ("FELanguage", "GetLanguage") => Ok(Value::Text("english".into())),
            ("Math", "floor") => Ok(Value::Number(
                args.first().unwrap_or(&Value::Undefined).number().floor(),
            )),
            ("Math", "ceil") => Ok(Value::Number(
                args.first().unwrap_or(&Value::Undefined).number().ceil(),
            )),
            ("Math", "random") => {
                self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
                Ok(Value::Number(self.random as f64 / (u32::MAX as f64 + 1.)))
            }
            _ => Err(format!(
                "Unimplemented front-end binding {kind}.{method}({args:?})"
            )),
        }
    }
}

impl Bindings {
    fn drain(&mut self, vm: &mut Vm) -> Result<(), String> {
        let mut count = 0;
        while let Some((object, offset)) = self.movie.pending.pop_front() {
            count += 1;
            if count > 4096 {
                return Err("Front-end timeline action limit".into());
            }
            if !self.movie.instances.contains_key(&object) {
                continue;
            }
            if offset == 0 {
                let class = self.movie.classes[&self.movie.instances[&object].character];
                vm.construct_on(object, class, self)
                    .map_err(|e| format!("constructor on {object}: {e}"))?;
            } else {
                let code = self
                    .movie
                    .actions
                    .get(&offset.to_string())
                    .ok_or("Missing front-end action block")?
                    .clone();
                vm.run_on(object, &code, self)
                    .map_err(|e| format!("action {offset} on object {object}: {e}"))?;
            }
        }
        Ok(())
    }
}

impl Runtime {
    pub fn load(source: &serde_json::Value) -> Result<Self, String> {
        let mut vm = Vm::new();
        for name in [
            "MovieClip",
            "Object",
            "Array",
            "TextFormat",
            "ScreenManager",
            "Audio",
            "Math",
            "FELanguage",
            "Game",
            "coremenu",
            "LetterBox",
            "Popup",
            "FEChallengeMap",
            "CIC",
            "Tricks",
            "HUDComponents",
            "TeammateSelect",
            "Mission",
        ] {
            let id = vm.object(ObjectKind::Native(name.into()));
            let prototype = vm.object(ObjectKind::Plain);
            vm.set(id, "prototype", Value::Object(prototype))?;
            vm.set(vm.global, name, Value::Object(id))?;
        }
        vm.set(vm.global, "_global", Value::Object(vm.global))?;
        for (name, value) in [
            ("Screen_EdgeOffset", 0.),
            ("Screen_Width", 1280.),
            ("Screen_Height", 720.),
            ("PlatformCurrent", 0.),
            ("Platform_XBOX360", 0.),
            ("Platform_PS3", 1.),
        ] {
            vm.set(vm.global, name, Value::Number(value))?;
        }
        let mut bindings = Bindings {
            movie: Movie::load(source)?,
            loaded: false,
            trace: Vec::new(),
            random: 0x534b4154,
            category: 0,
            selection: 0,
            dimmer: false,
            options: vec![
                MenuOption {
                    title: "ID_GAMESETTINGS_DIFFICULTY_SETTINGS".into(),
                    kind: "selector".into(),
                    value: "#Easy".into(),
                    description: "#Choose Easy, Normal or Hardcore skating.".into(),
                    integer: 0,
                    enabled: true,
                },
                MenuOption::option("#PC Graphics", "#Display and rendering settings."),
            ],
            options_title: "ID_GAMESETTINGS_TITLE".into(),
            callbacks: Vec::new(),
            popup_title: "#Not available".into(),
            popup_description: "#The trick guide is not available in this build.".into(),
            buttons: serde_json::from_value(
                source
                    .get("buttons")
                    .cloned()
                    .unwrap_or(serde_json::json!({})),
            )
            .map_err(|e| e.to_string())?,
        };
        for offset in source["initializers"]
            .as_array()
            .ok_or("Front-end initializer table absent")?
        {
            let offset = offset
                .as_u64()
                .ok_or("Invalid front-end initializer")?
                .to_string();
            let code = bindings.movie.actions[&offset].clone();
            vm.run(&code, &mut bindings)
                .map_err(|e| format!("initializer {offset}: {e}"))?;
        }
        bindings.movie.initialize(&mut vm)?;
        bindings
            .movie
            .refresh_dimensions(&mut vm, bindings.movie.root)?;
        let mut result = Self {
            vm,
            bindings,
            controller: usize::MAX,
        };
        result.vm.begin_update();
        result.drain()?;
        result.controller = match result.vm.get(result.bindings.movie.root, "screen") {
            Value::Object(id) => id,
            _ if source["bundle"].as_str() == Some("data/fe/source/screens/main/dimmer") => {
                result.bindings.movie.root
            }
            _ => return Err("Original screen did not create its controller".into()),
        };
        // ScreenManager activates the clip after OnLoaded. Several retail
        // popup constructors intentionally hide it while building controls.
        result
            .vm
            .set(result.bindings.movie.root, "_visible", Value::Bool(true))?;
        Ok(result)
    }
    pub fn drain(&mut self) -> Result<(), String> {
        self.bindings.drain(&mut self.vm)
    }
    pub fn call(&mut self, method: &str, args: Vec<Value>) -> Result<Value, String> {
        self.vm.begin_update();
        let value = self
            .vm
            .call_method(self.controller, method, args, &mut self.bindings)?;
        self.drain()?;
        Ok(value)
    }
    pub fn update(&mut self) -> Result<(), String> {
        self.vm.begin_update();
        self.bindings.movie.advance(&mut self.vm)?;
        self.drain()?;
        self.vm
            .collect(self.bindings.movie.instances.keys().copied())?;
        Ok(())
    }
}
