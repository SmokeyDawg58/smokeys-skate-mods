use super::{Lua, LuaResult, NativeCall, Proto, Value};
use std::sync::Arc;

const fn abc(op: u32, a: u32, b: u32, c: u32) -> u32 {
    op | (a << 6) | (c << 14) | (b << 23)
}
const fn abx(op: u32, a: u32, bx: u32) -> u32 {
    op | (a << 6) | (bx << 14)
}
#[expect(clippy::cast_sign_loss, reason = "biased sBx")]
const fn asbx(op: u32, a: u32, sbx: i32) -> u32 {
    abx(op, a, (sbx + 131_071) as u32)
}
const K: u32 = 256;

fn proto(code: Vec<u32>, constants: Vec<Value>, params: usize, max_stack: usize) -> Proto {
    Proto {
        code,
        constants,
        protos: Vec::new(),
        parameter_count: params,
        vararg: false,
        max_stack,
        upvalue_count: 0,
        source: Arc::from("test"),
        line_defined: 0,
    }
}

fn run(lua: &mut Lua<()>, p: Proto, args: &[Value]) -> LuaResult<Vec<Value>> {
    let f = Value::Function(lua.create_closure(Arc::new(p), Vec::new()));
    lua.call(&mut (), &f, args)
}

fn n(x: f32) -> Value {
    Value::Number(x)
}

#[test]
fn arithmetic_uses_float_semantics() {
    let mut lua = Lua::<()>::new();
    // return 1 + 2 * 3, 7 / 2, 0.1 + 0.2
    let p = proto(
        vec![
            abc(14, 0, K + 1, K + 2),
            abc(12, 0, K, 0),
            abc(15, 1, K + 3, K + 1),
            abc(12, 2, K + 4, K + 5),
            abc(30, 0, 4, 0),
        ],
        vec![n(1.0), n(2.0), n(3.0), n(7.0), n(0.1), n(0.2)],
        0,
        4,
    );
    let r = run(&mut lua, p, &[]).unwrap();
    assert!(r[0].raw_equal(&n(7.0)));
    assert!(r[1].raw_equal(&n(3.5)));
    assert!(r[2].raw_equal(&n(0.1f32 + 0.2f32)));
    assert_eq!(lua.display(&r[2]), "0.30000001192093");
}

#[test]
fn numeric_for_loop_and_setlist() {
    let mut lua = Lua::<()>::new();
    // local s = 0; for i = 1, 10 do s = s + i end; local t = {s, 2, 3}; return s, #t
    let p = proto(
        vec![
            abx(1, 0, 0),
            abx(1, 1, 1),
            abx(1, 2, 2),
            abx(1, 3, 1),
            asbx(32, 1, 1),
            abc(12, 0, 0, 4),
            asbx(31, 1, -2),
            abc(10, 5, 3, 0),
            abc(0, 6, 0, 0),
            abx(1, 7, 3),
            abx(1, 8, 4),
            abc(34, 5, 3, 1),
            abc(20, 6, 5, 0),
            abc(30, 0, 1 + 1, 0),
        ],
        vec![n(0.0), n(1.0), n(10.0), n(2.0), n(3.0)],
        0,
        9,
    );
    let r = run(&mut lua, p, &[]).unwrap();
    assert!(r[0].raw_equal(&n(55.0)));
}

fn add_one(_: &mut Lua<()>, _: &mut (), call: &NativeCall) -> LuaResult<Vec<Value>> {
    Ok(vec![n(call.arg(0).as_number().unwrap() + 1.0)])
}

#[test]
fn natives_globals_and_string_format() {
    let mut lua = Lua::<()>::new();
    let f = lua.create_native("add_one", add_one, Vec::new());
    lua.set_global("add_one", f);
    // return string.format("%d %s %5.2f|%-3d|", add_one(4), "x", 2.5, 7)
    let p = proto(
        vec![
            abx(5, 0, 0),
            abc(6, 0, 0, K + 1),
            abx(1, 1, 2),
            abx(5, 2, 3),
            abx(1, 3, 4),
            abc(28, 2, 2, 2),
            abx(1, 3, 5),
            abx(1, 4, 6),
            abx(1, 5, 7),
            abc(28, 0, 6, 2),
            abc(30, 0, 2, 0),
        ],
        vec![
            Value::string("string"),
            Value::string("format"),
            Value::string("%d %s %5.2f|%-3d|"),
            Value::string("add_one"),
            n(4.0),
            Value::string("x"),
            n(2.5),
            n(7.0),
        ],
        0,
        6,
    );
    let r = run(&mut lua, p, &[]).unwrap();
    assert_eq!(lua.display(&r[0]), "5 x  2.50|7  |");
}

#[test]
fn indexing_nil_reports_the_field() {
    let mut lua = Lua::<()>::new();
    // return HUD2.Show
    let p = proto(vec![abx(5, 0, 0), abc(6, 0, 0, K + 1), abc(30, 0, 2, 0)], vec![Value::string("HUD2"), Value::string("Show")], 0, 2);
    let e = run(&mut lua, p, &[]).unwrap_err();
    assert_eq!(e.message, "attempt to index global 'HUD2' (a nil value)");
    // HUD2 = {}; HUD2.Show()
    let t = lua.create_table();
    lua.set_global("HUD2", Value::Table(t));
    let p = proto(
        vec![abx(5, 0, 0), abc(6, 0, 0, K + 1), abc(28, 0, 1, 1), abc(30, 0, 1, 0)],
        vec![Value::string("HUD2"), Value::string("Show")],
        0,
        2,
    );
    let e = run(&mut lua, p, &[]).unwrap_err();
    assert_eq!(e.message, "attempt to call field 'Show' (a nil value)");
    assert_eq!(lua.top, 0);
}

#[test]
fn table_sort_matches_lua_quicksort() {
    let mut lua = Lua::<()>::new();
    let t = lua.create_table();
    let values = [5.0, 3.0, 9.0, 1.0, 7.0, 2.0, 8.0, 6.0, 4.0, 0.0];
    for (i, v) in values.iter().enumerate() {
        lua.table_mut(t).set_int(i + 1, n(*v));
    }
    let sort = lua.index(&mut (), &lua.get_global("table"), &Value::string("sort")).unwrap();
    lua.call(&mut (), &sort, &[Value::Table(t)]).unwrap();
    let sorted: Vec<f32> = (1..=10).map(|i| lua.table(t).get_int(i).as_number().unwrap()).collect();
    assert_eq!(sorted, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]);
}

#[test]
fn closures_share_upvalues() {
    let mut lua = Lua::<()>::new();
    // local c = 0; local inc = function() c = c + 1 return c end; inc(); return inc()
    let mut inner = proto(vec![abc(4, 0, 0, 0), abc(12, 0, 0, K), abc(8, 0, 0, 0), abc(30, 0, 2, 0)], vec![n(1.0)], 0, 2);
    inner.upvalue_count = 1;
    let mut outer = proto(
        vec![
            abx(1, 0, 0),
            abx(36, 1, 0),
            abc(0, 0, 0, 0),
            abc(0, 2, 1, 0),
            abc(28, 2, 1, 1),
            abc(0, 2, 1, 0),
            abc(28, 2, 1, 2),
            abc(30, 2, 2, 0),
        ],
        vec![n(0.0)],
        0,
        3,
    );
    outer.protos.push(Arc::new(inner));
    let r = run(&mut lua, outer, &[]).unwrap();
    assert!(r[0].raw_equal(&n(2.0)));
}

#[test]
fn collector_frees_unreachable_tables() {
    let mut lua = Lua::<()>::new();
    let baseline = lua.live_objects();
    for _ in 0..100 {
        lua.create_table();
    }
    let kept = lua.create_table();
    lua.set_global("kept", Value::Table(kept));
    lua.collect_garbage();
    assert_eq!(lua.live_objects(), baseline + 1);
    assert!(lua.get_global("kept").raw_equal(&Value::Table(kept)));
}

/// Every handler chunk in both vaults loads, and running each main function
/// defines exactly its `h_<collection>` global.
#[test]
#[ignore = "requires original external game data via SKATE2_GAME_ROOT"]
fn all_shipped_chunks_load_and_define_handlers() {
    use skate2_formats::mount::{MountSet, MountedBig, SKATE2_ARCHIVES};
    let root = std::env::var("SKATE2_GAME_ROOT").expect("SKATE2_GAME_ROOT");
    let archive = SKATE2_ARCHIVES.iter().find(|a| a.relative_path == "data\\db\\db.big").unwrap();
    let mut mounts = MountSet::new();
    mounts.push(MountedBig::open(archive.path_under(std::path::Path::new(&root)), archive.use_external_header).unwrap());
    let mut total = 0;
    for name in ["data\\db\\ChallengeCore.bin", "data\\db\\progressioncore.bin"] {
        let bytes = mounts.read_path(name).unwrap();
        let mut lua = Lua::<()>::new();
        let mut at = 0;
        while let Some(found) = bytes[at..].windows(12).position(|w| w == skate2_formats::lua_chunk::HEADER) {
            let start = at + found;
            let (_, used) = skate2_formats::lua_chunk::parse(&bytes[start..]).unwrap();
            let main = lua.load(&bytes[start..start + used], name).unwrap();
            lua.call(&mut (), &main, &[]).unwrap();
            total += 1;
            at = start + used;
        }
        let globals = lua.globals();
        let mut handlers = 0;
        let mut key = Value::Nil;
        while let Some((k, v)) = lua.table(globals).next(&key).unwrap() {
            if k.as_str().is_some_and(|s| s.as_bytes().starts_with(b"h_")) {
                assert!(matches!(v, Value::Function(_)));
                handlers += 1;
            }
            key = k;
        }
        // Challenge handlers are named by their collection; progression
        // handlers by StringHash64 of the message class, so only six names.
        let expected = if name.contains("progression") { 6 } else { 627 };
        assert_eq!(handlers, expected, "{name}");
    }
    assert_eq!(total, 627 + 910);
}
