use html_to_string_macro::html_to_string;

pub fn big_table(b: &mut criterion::Bencher<'_>, size: &usize) {
    let mut table = Vec::with_capacity(*size);
    for _ in 0..*size {
        let mut inner = Vec::with_capacity(*size);
        for i in 0..*size {
            inner.push(i);
        }
        table.push(inner);
    }
    b.iter(|| {
        let ctx = BigTable { table: &table };
        html_to_string! {
            <table>
            {
                table.iter().map(|row| {
                    html_to_string! {
                        <tr>
                        {
                            row.iter().map(|col| {
                                html_to_string! {
                                    <td>{ col }</td>
                                }
                            }).collect::<String>()
                        }
                        </tr>
                    }
                }).collect::<String>()
            }
            </table>
        };
    });
}

pub fn teams(b: &mut criterion::Bencher<'_>) {
    let teams = Teams {
        year: 2015,
        teams: vec![
            Team {
                name: "Jiangsu".into(),

                score: 43,
            },
            Team {
                name: "Beijing".into(),
                score: 27,
            },
            Team {
                name: "Guangzhou".into(),
                score: 22,
            },
            Team {
                name: "Shandong".into(),
                score: 12,
            },
        ],
    };
    b.iter(|| {
        html_to_string! {
            <html>
            <head>
              <title>{ teams.year }</title>
            </head>
            <body>
              <h1>"CSL "{ teams.year }</h1>
              <ul>
              {
                  teams.teams.iter().enumerate().map(|(i, team)| {
                      html_to_string! {
                        <li class=format!("{}", if i == 0 { "champion" } else { "" })><b>{ &team.name }</b>": "{ &team.score }</li>
                      }
                  }).collect::<String>()
              }
              </ul>
            </body>
          </html> 
        };
    });
}

struct BigTable<'a> {
    table: &'a [Vec<usize>],
}

struct Teams {
    year: u16,
    teams: Vec<Team>,
}

struct Team {
    name: String,
    score: u8,
}
