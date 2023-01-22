#[macro_use] extern crate rocket;
use rocket::http::{Status, ContentType};
use rocket::form::{self, Error, Form, Contextual, FromForm, FromFormField, Context};
use rocket::fs::{FileServer, TempFile, relative};
use rocket::State;
use rocket_dyn_templates::Template;
use rocket_dyn_templates::tera;
use rocket_dyn_templates::context;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use rocket::serde::{Serialize, Deserialize};
use serde_json::json;
use rocket::response::status;

#[derive(Debug, FromForm)]
struct Submit<'v> {
    #[field(validate = check())]
    file: TempFile<'v>,
}

fn check<'v>(file: &TempFile) -> form::Result<'v, ()> {
    //check for extension and 3 backticks
    Ok(())
}

#[get("/")]
fn index() -> Template {
    Template::render("index", &Context::default())
}

struct SubmissionNumber{
    n: AtomicUsize,
}

fn write_json<T: Serialize>(path: &str, data: &T) -> Result<(), String> {
    match serde_json::to_string_pretty(data) {
        Ok(string) => {
            if let Err(error) =  std::fs::write(path, string) {
                return Err(format!("{error}"));
            }
        },
        Err(error) => return Err(format!("{error}")),
    }
    Ok(())
}

#[post("/", data = "<form>")]
async fn submit<'r>(mut form: Form<Contextual<'r, Submit<'r>>>, submission_number: &State<SubmissionNumber>) -> (Status, Template) {
    let template = match form.value {
        Some(ref mut submission) => {
            println!("submission: {:#?}", submission);
            let mut errcontext = tera::Context::new();
            match get_ext(&submission.file) {
                Ok(ext) => {
                    let filename = format!("{:010}", submission_number.n.load(Ordering::Relaxed));
                    let sub = new_submissionstruct(ext, filename.clone(), String::from("testing task"), 
                                                   format!("{:?}", chrono::offset::Local::now()), String::from("me"));
                    let json = json!(sub);
                    match submission.file.persist_to("./submissions/".to_owned() + &filename + ".txt").await {
                        Ok(()) => {
                            match write_json(&("./submissions/".to_owned() + &filename + ".json"), &json) {
                                Ok(()) => {
                                    let cnt = submission_number.n.fetch_add(1, Ordering::Relaxed);
                                    view_submission(filename)
                                },
                                Err(error) => {
                                    errcontext.insert("during", "writing json");
                                    errcontext.insert("error", &format!("{error}"));
                                    Template::render("error", errcontext.into_json())
                                },
                            }
                        },
                        Err(error) => {
                            errcontext.insert("during", "writing code file");
                            errcontext.insert("error", &format!("{error}"));
                            Template::render("error", errcontext.into_json())
                        },
                    }
                },
                Err(error) => {
                    errcontext.insert("during", "getting extension");
                    errcontext.insert("error", &format!("{error}"));
                    Template::render("error", errcontext.into_json())
                },
            }
        },
        None => Template::render("index", &form.context),
    };
    (form.context.status(), template)
}

#[get("/view/<submission_number>")]
fn view_submission(submission_number: String) -> Template {
    let j = read_submission_json("./submissions/".to_owned() + &submission_number + ".json");
    let mut s = match std::fs::read_to_string(&("./submissions/".to_owned() + &submission_number + ".txt")) {
        Ok(code) => code,
        Err(error) => format!("{error}"),
    };
    s = comrak::markdown_to_html(&("```".to_owned() + &s + "```"), &comrak::ComrakOptions::default());
    Template::render("view", &context!{json: j, code: s})
}

fn get_ext(f: &TempFile) -> Result<String, String> {
    match f.raw_name() {
        Some(raw) => {
            match raw.dangerous_unsafe_unsanitized_raw().as_str().rfind('.') {
                Some(pos) => Ok(raw.dangerous_unsafe_unsanitized_raw().as_str()[pos..].to_owned()),
                None => Err(String::from("bad file naming")),
            }
        },
        None => Err(String::from("couldn't get raw name of file")),
    }
}

fn get_lastmodified() -> u64 {
    //https://stackoverflow.com/questions/74283613/rust-i-want-to-get-last-modified-file-in-a-dir
    //for loop until reach a file that has a number as its stem
    0
}

fn get_submissionnumber() -> AtomicUsize {
    //scan submission folder, return max + 1
    AtomicUsize::new(0)
}

fn read_submission_json(filepath: String) -> Submission {
    match std::fs::read_to_string(&filepath) {
        Ok(data) => {
            match serde_json::from_str::<Submission>(&data) {
                Ok(sub) => sub,
                Err(error) => {
                    new_submissionstruct(String::from("err"), String::from("err"), String::from("err"), 
                                         String::from("err"), format!("{error}"))
                },
            }
        },
        Err(error) => {
            new_submissionstruct(String::from("err"), String::from("err"), String::from("err"), 
                                 String::from("err"), format!("{error}"))
        },
    }
}

#[derive(Serialize, Deserialize)]
struct Submission{
    Language: String,
    Submission: String,
    Task: String,
    Time: String,
    User: String,
}

fn new_submissionstruct(Language: String, Submission: String, Task: String, Time: String, User: String) -> Submission {
    Submission {Language, Submission, Task, Time, User}
}

static subperpage: usize = 20;
#[get("/<page>")]
fn submissions<'r>(page: usize, submission_number: &State<SubmissionNumber>) -> Template {
    let cur = submission_number.n.load(Ordering::Relaxed);
    let mut table = String::from("");
    let (mut start, mut end) = (0, 0);
    if cur > subperpage * (page + 1) {start = cur - subperpage * (page + 1);}
    if cur > subperpage * (page) {end = cur - subperpage * page;}
    for i in (start..end).rev() {
        let j = read_submission_json("./submissions/".to_owned() + format!("{:010}", i).as_str() + ".json");
        table += &("{Submission:\"".to_owned() + j.Submission.as_str() + "\",Task:\"" + j.Task.as_str() + "\",Language:\"" + 
                 j.Language.as_str() + "\",User:\"" + j.User.as_str() + "\",Time:\"" + j.Time.as_str() + "\"},\n");
    }
    let mut context = tera::Context::new();
    context.insert("table", &table); 
    Template::render("submissions", context.into_json())
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, submit])
        .mount("/submissions", routes![submissions, view_submission])
        .manage(SubmissionNumber{n: get_submissionnumber()})
        .mount("/", FileServer::from(relative!("/static")))
        .attach(Template::fairing())
}
