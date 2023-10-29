use askama::Template;
use axum::{routing::get, Router};

#[derive(Template)]
#[template(path = "pages/resume.html")]
struct ResumePage<'a> {
    education: &'a [Education<'a>],
    skills: &'a [Skill<'a>],
    projects: &'a [Project<'a>],
    experiences: &'a [Experience<'a>],
}

#[derive(Debug, Clone, Copy, Default)]
struct Education<'a> {
    what: &'a str,
    r#where: &'a str,
    when: &'a str,
}

#[derive(Debug, Clone, Copy, Default)]
struct Skill<'a> {
    title: &'a str,
    list: &'a [&'a str],
}

#[derive(Debug, Clone, Copy, Default)]
struct Project<'a> {
    title: &'a str,
    list: &'a [&'a str],
}

#[derive(Debug, Clone, Copy, Default)]
struct Experience<'a> {
    r#where: &'a str,
    role: &'a str,
    location: &'a str,
    when: &'a str,
    list: &'a [&'a str],
}

const EDUCATION: &[Education] = &[
    Education {
        what: "Bachelor of Engineering with Honours – Software Engineering",
        r#where: "Victoria University of Wellington",
        when: "2020 – 2023",
    },
    Education {
        what: "Star Program – Advancing in Mathematical Sciences",
        r#where: "University of Canterbury",
        when: "2019 (full year)",
    },
    Education {
        what: "NCEA Level 3 with Excellence Endorsement",
        r#where: "Mount Maunganui College",
        when: "2015 – 2019",
    },
];

const SKILLS: &[Skill] = &[
    Skill {
        title: "Languages",
        list: &[
            "Rust",
            "HTML",
            "CSS",
            "JavaScipt",
            "TypeScript",
            "SQL",
            "Java",
            "C",
            "C++",
            "C#",
            "Bash",
            "Lua",
            "Ruby",
        ],
    },
    Skill {
        title: "Frameworks / Tools",
        list: &[
            "Git",
            "PostgreSQL",
            "MySQL",
            "Redis",
            "Docker",
            "NodeJS",
            "ExpressJS",
            "React",
            "Unity Engine",
        ],
    },
    Skill {
        title: "Other Technology",
        list: &["Cloudflare", "AWS", "Proxmox"],
    },
];

const PROJECTS: &[Project] = &[
    Project {
        title: "Ipipiri Digital Trails: Augmented Reality Experience",
        list: &[
            "I worked with the Russell Museum to develop an augmented reality application that allows users to explore the history of Russell.",
            "My contribution was the development of the augmented reality elements and building the Android application.",
            "Developed using: C#, Unity Engine, Android SDK.",
        ],
    },
    Project {
        title: "Home Lab",
        list: &[
            "I maintain a small home lab with two servers for the purpose of learning server technology.",
            "I also use it to manage all my home networking, host internal services, and create isolated development environments.",
            "Technologies used: Proxmox, Docker.",
        ],
    },
    Project {
        title: "Recipe Book",
        list: &[
            "Web application to manage recipes in the browser.",
            "Integrates with the ChatGPT for automatically creating recipes.",
            "Developed using: Rust, Leptos, OpenAI API.",
        ],
    },
    Project {
        title: "Wedding/Event Venue Website",
        list: &[
            "Website for a weddings and events venue. It was a full-stack application that used basic HTML and CSS for the front-end and PHP and MySQL for the back-end.",
            "Implemented a basic content management system that allowed the content of the website to be changed without requiring changes to the code.",
            "Developed using: PHP, HTML, CSS, SQL.",
        ],
    },
];

const EXPERIENCES: &[Experience] = &[
    Experience {
        r#where: "McDonald's",
        role: "Crew Member",
        location: "Wellington, NZ",
        when: "2021 – 2022 (4 months)",
        list: &[
            "My role was to work the kitchen half of the restaurant.",
            "This involved being apart in a team to cook and complete 30+ orders per hour.",
        ],
    },
    Experience {
        r#where: "Summer House Weddings & Events",
        role: "Various Roles",
        location: "Tauranga, NZ",
        when: "2015 – 2019",
        list: &[
            "I developed and managed the website for Summer House Weddings & Events.",
            "During an event I also worked as security and parking. For some events I worked tending the bar.",
        ],
    },
];

pub fn router() -> Router {
    Router::new().route(
        "/",
        get(async || ResumePage {
            education: EDUCATION,
            skills: SKILLS,
            projects: PROJECTS,
            experiences: EXPERIENCES,
        }),
    )
}
