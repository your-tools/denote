use time::macros::format_description;
use time::OffsetDateTime;

fn main() {
    let now = OffsetDateTime::now_utc();
    let format = format_description!("[year][month][day]T[hour]:[minute]:[second]");
    let formatted_date = now
        .format(&format)
        .expect("now should be formatted correctly");

    let template = format!(
        r#"---
date: {formatted_date}
title:
tags: []
    "#
    );
    println!("{template}")
}
