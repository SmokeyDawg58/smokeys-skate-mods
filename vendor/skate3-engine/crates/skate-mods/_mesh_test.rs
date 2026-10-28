use mlua::{Lua, LuaSerdeExt, StdLib, LuaOptions};
use skate_mods::vm::Command;

fn main() {
    let lua = Lua::new_with(StdLib::ALL, LuaOptions::default()).unwrap();
    let cases = [
        ("nested_uvs", r#"{kind="graphics_mesh_buffer_write",key="t",data={positions={{0,0,0},{1,0,0},{0,1,0}},indices={0,1,2},uvs={{0,0},{1,0},{0,1}}}}"#),
        ("flat_uvs", r#"{kind="graphics_mesh_buffer_write",key="t",data={positions={{0,0,0},{1,0,0},{0,1,0}},indices={0,1,2},uvs={0,0,1,0,0,1}}}"#),
        ("skid_like", r#"{kind="graphics_mesh_buffer_write",key="t",data={positions={{1,2,3},{4,5,6},{7,8,9},{10,11,12}},indices={0,1,2,1,3,2},uvs={{0,0},{1,0},{0,0.25},{1,0.25}}}}"#),
    ];
    for (name, src) in cases {
        let v = lua.load(&format!("return {}", src)).eval::<mlua::Value>().unwrap();
        match lua.from_value::<Command>(v) {
            Ok(_) => println!("{}: OK", name),
            Err(e) => println!("{}: ERR {}", name, e),
        }
    }
}
