//! dm-hko-v1, ISC year table adapted from yize/solarlunar b1d4328.
//! License and attribution: ../../LICENSE-lunar.
use chrono::{NaiveDate, Duration};
use serde_json::{json, Value};
pub const VERSION: &str = "dm-hko-v1";
const TABLE: [u32;201] = [
19416,19168,42352,21717,53856,55632,91476,22176,39632,21970,
19168,42422,42192,53840,119381,46400,54944,44450,38320,84343,
18800,42160,46261,27216,27968,109396,11104,38256,21234,18800,
25958,54432,59984,28309,23248,11104,100067,37600,116951,51536,
54432,120998,46416,22176,107956,9680,37584,53938,43344,46423,
27808,46416,86869,19872,42416,83315,21168,43432,59728,27296,
44710,43856,19296,43748,42352,21088,62051,55632,23383,22176,
38608,19925,19152,42192,54484,53840,54616,46400,46752,103846,
38320,18864,43380,42160,45690,27216,27968,44870,43872,38256,
19189,18800,25776,29859,59984,27480,23232,43872,38613,37600,
51552,55636,54432,55888,30034,22176,43959,9680,37584,51893,
43344,46240,47780,44368,21977,19360,42416,86390,21168,43312,
31060,27296,44368,23378,19296,42726,42208,53856,60005,54576,
23200,30371,38608,19195,19152,42192,118966,53840,54560,56645,
46496,22224,21938,18864,42359,42160,43600,111189,27936,44448,
84835,37744,18936,18800,25776,92326,59984,27296,108228,43744,
37600,53987,51552,54615,54432,55888,23893,22176,42704,21972,
21200,43448,43344,46240,46758,44368,21920,43940,42416,21168,
45683,26928,29495,27296,44368,84821,19296,42352,21732,53600,
59752,54560,55968,92838,22224,19168,43476,41680,53584,62034,54560];

pub fn months(year:i32) -> Vec<(u32,bool,u32)> {
    if !(1900..=2100).contains(&year) { return vec![]; }
    let info = TABLE[(year-1900) as usize];
    let mut result = vec![];
    for m in 1..=12 { result.push((m,false,29+u32::from(info & (0x10000 >> m) != 0)));
        if info & 15 == m { result.push((m,true,29+u32::from(info & 0x10000 != 0))); } }
    result
}
pub fn to_solar(value:&Value) -> Option<String> {
    let year = i32::try_from(value["year"].as_i64()?).ok()?;
    let month = u32::try_from(value["month"].as_u64()?).ok()?;
    let day = u32::try_from(value["day"].as_u64()?).ok()?;
    let leap = value["leap"].as_bool()?;
    if !(1900..=2100).contains(&year) { return None; }
    let mut offset:u32 = (1900..year).map(|y|months(y).iter().map(|m|m.2).sum::<u32>()).sum();
    for (m,l,len) in months(year) { if (m,l)==(month,leap) {
        if day == 0 || day > len {return None;}
        return Some((NaiveDate::from_ymd_opt(1900,1,31)? + Duration::days((offset+day-1) as i64)).format("%Y-%m-%d").to_string());
    } offset+=len; } None
}
pub fn from_solar(date:&str) -> Option<Value> {
    let date = NaiveDate::parse_from_str(date,"%Y-%m-%d").ok()?;
    let mut offset = (date - NaiveDate::from_ymd_opt(1900,1,31)?).num_days();
    if offset < 0 {return None;}
    for year in 1900..=2100 { for (month,leap,len) in months(year) {
        if offset < len as i64 {return Some(json!({"year":year,"month":month,"day":offset+1,"leap":leap}));}
        offset -= len as i64;
    }} None
}
pub fn validate(event:&Value) -> Result<(), &'static str> {
    if event.get("calendar").is_some_and(|v|!v.is_string()) {return Err("历法字段无效");}
    match event.get("calendar").and_then(Value::as_str) {
        None | Some("solar") => { if event.get("lunarDate").is_some_and(|v| !v.is_null()) {return Err("公历不能保留农历日期字段");} }
        Some("lunar") => {
            if event["lunarTableVersion"] != VERSION || event["lunarLeapPolicy"] != "regularFallback" || event["lunarShortMonthPolicy"] != "lastDay"
                || to_solar(&event["lunarDate"]).as_deref() != event["date"].as_str() {return Err("农历日期、换算版本或对应公历日期无效");}
        }
        _ => return Err("历法无效"),
    } Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_table_as_wrist_and_all_lunar_dates_roundtrip() {
        let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../daymatter/src/components/dateUtils.js")).unwrap();
        let table = source.split("const LUNAR_TABLE = ").nth(1).unwrap().split(']').next().unwrap().to_string()+"]";
        let wrist:Vec<u32> = serde_json::from_str(&table).unwrap();
        assert_eq!(wrist.as_slice(),TABLE);
        for year in 1900..=2100 {for (month,leap,len) in months(year) {for day in 1..=len {
            let value=json!({"year":year,"month":month,"day":day,"leap":leap});
            assert_eq!(from_solar(&to_solar(&value).unwrap()).unwrap(),value);
        }}}
    }
}
pub fn fields(date:Value) -> Value {json!({"calendar":"lunar","lunarDate":date,"lunarTableVersion":VERSION,
    "lunarLeapPolicy":"regularFallback","lunarShortMonthPolicy":"lastDay"})}
pub fn prepare(draft:&super::state::EventData) -> Result<super::state::EventData,&'static str> {
    let mut result = draft.clone();
    if result.extensions.get("calendar").and_then(Value::as_str) == Some("lunar") {
        if let Some(text) = result.extensions.get("lunarInput").and_then(Value::as_str) {
            let parts: Vec<_> = text.trim().split('-').collect();
            if parts.len()!=3 {return Err("农历日期格式为 YYYY-MM-DD，闰月另选");}
            let date = json!({"year":parts[0].parse::<i32>().map_err(|_|"农历年份无效")?,
                "month":parts[1].parse::<u32>().map_err(|_|"农历月份无效")?,"day":parts[2].parse::<u32>().map_err(|_|"农历日期无效")?,
                "leap":result.extensions.get("lunarDate").and_then(|v|v.get("leap")).and_then(Value::as_bool).unwrap_or(false)});
            result.extensions.insert("lunarDate".into(),date);
        }
        result.time = to_solar(&result.extensions["lunarDate"]).ok_or("农历日期不存在，或超出1900～2100年")?;
    }
    result.extensions.remove("lunarInput");
    validate(&super::backup::event_value(&result))?;
    Ok(result)
}
pub fn occurrence(event:&super::state::EventData, today:&str) -> Option<(String,Option<i32>)> {
    let value = super::backup::event_value(event);
    validate(&value).ok()?;
    let source = &value["lunarDate"];
    if value["repeat"] != "yearly" {return Some((event.time.clone(),None));}
    let start = source["year"].as_i64()? as i32;
    let today_year = from_solar(today)?["year"].as_i64()? as i32;
    let month = source["month"].as_u64()? as u32;
    for year in start.max(today_year)..=2100 {
        let months = months(year);
        let target = months.iter().find(|m|m.0==month && m.1==source["leap"].as_bool().unwrap())
            .or_else(||months.iter().find(|m|m.0==month && !m.1))?;
        let date = to_solar(&json!({"year":year,"month":month,"day":(source["day"].as_u64()? as u32).min(target.2),"leap":target.1}))?;
        if date.as_str() >= today {return Some((date,Some(year-start)));}
    } None
}
