use crate::entry::FileEntry;
use crate::icons::{icon_for, resolved_category};
use crate::render::RenderCtx;
use crate::util::{format_time, human_size};

#[cfg(unix)]
fn permission_string(entry: &FileEntry, ctx: &RenderCtx) -> String {
    let mode = entry.mode;
    let kind = if entry.is_dir {
        'd'
    } else if entry.is_symlink {
        'l'
    } else {
        '-'
    };

    let bits = [
        (0o400, 'r', "permission_read"),
        (0o200, 'w', "permission_write"),
        (0o100, 'x', "permission_execute"),
        (0o040, 'r', "permission_read"),
        (0o020, 'w', "permission_write"),
        (0o010, 'x', "permission_execute"),
        (0o004, 'r', "permission_read"),
        (0o002, 'w', "permission_write"),
        (0o001, 'x', "permission_execute"),
    ];

    let mut s = String::new();
    s.push(kind);
    for (mask, ch, cat) in bits {
        if mode & mask != 0 {
            s.push_str(&crate::colors::paint(
                &ch.to_string(),
                cat,
                ctx.theme,
                ctx.color_enabled,
            ));
        } else {
            s.push_str(&crate::colors::paint(
                "-",
                "permission_none",
                ctx.theme,
                ctx.color_enabled,
            ));
        }
    }
    s
}

#[cfg(not(unix))]
fn permission_string(entry: &FileEntry, ctx: &RenderCtx) -> String {
    let kind = if entry.is_dir {
        'd'
    } else if entry.is_symlink {
        'l'
    } else {
        '-'
    };
    let (r_cat, w_cat) = ("permission_read", "permission_write");
    let mut s = String::new();
    s.push(kind);
    s.push_str(&crate::colors::paint(
        "r",
        r_cat,
        ctx.theme,
        ctx.color_enabled,
    ));
    if entry.readonly {
        s.push_str(&crate::colors::paint(
            "-",
            "permission_none",
            ctx.theme,
            ctx.color_enabled,
        ));
    } else {
        s.push_str(&crate::colors::paint(
            "w",
            w_cat,
            ctx.theme,
            ctx.color_enabled,
        ));
    }
    s.push_str(&crate::colors::paint(
        if entry.is_executable() { "x" } else { "-" },
        "permission_execute",
        ctx.theme,
        ctx.color_enabled,
    ));
    s
}

#[cfg(all(unix, not(target_os = "android")))]
fn owner_group(entry: &FileEntry) -> (String, String) {
    let user = users::get_user_by_uid(entry.uid)
        .map(|u| u.name().to_string_lossy().to_string())
        .unwrap_or_else(|| entry.uid.to_string());
    let group = users::get_group_by_gid(entry.gid)
        .map(|g| g.name().to_string_lossy().to_string())
        .unwrap_or_else(|| entry.gid.to_string());
    (user, group)
}

// Android's bionic libc doesn't expose a conventional multi-user
// passwd/group database the way glibc/musl/BSD libc do, and the `users`
// crate isn't guaranteed to build cleanly against it. Show the raw
// numeric uid/gid instead of pulling in that dependency for this target
// (this also covers Termux, which targets `android` the same way).
#[cfg(target_os = "android")]
fn owner_group(entry: &FileEntry) -> (String, String) {
    (entry.uid.to_string(), entry.gid.to_string())
}

#[cfg(not(unix))]
fn owner_group(_entry: &FileEntry) -> (String, String) {
    ("-".to_string(), "-".to_string())
}

pub fn render(entries: &[FileEntry], ctx: &RenderCtx) {
    if entries.is_empty() {
        return;
    }

    let size_strs: Vec<String> = entries
        .iter()
        .map(|e| {
            if e.is_dir {
                "-".to_string()
            } else {
                human_size(e.size)
            }
        })
        .collect();
    let size_width = size_strs.iter().map(|s| s.len()).max().unwrap_or(1);

    let (user_w, group_w) = if cfg!(unix) {
        let pairs: Vec<(String, String)> = entries.iter().map(owner_group).collect();
        (
            pairs.iter().map(|(u, _)| u.len()).max().unwrap_or(1),
            pairs.iter().map(|(_, g)| g.len()).max().unwrap_or(1),
        )
    } else {
        (1, 1)
    };

    for entry in entries {
        let perms = permission_string(entry, ctx);
        let (user, group) = owner_group(entry);
        let size = if entry.is_dir {
            "-".to_string()
        } else {
            human_size(entry.size)
        };
        let date = entry
            .modified
            .map(format_time)
            .unwrap_or_else(|| "-".to_string());

        let icon = if ctx.icons_enabled {
            format!("{} ", icon_for(entry, ctx.theme))
        } else {
            String::new()
        };

        let mut label = entry.name.clone();
        if entry.is_dir {
            label.push('/');
        }
        if let Some(target) = &entry.symlink_target {
            label.push_str(" -> ");
            label.push_str(&target.to_string_lossy());
        }

        let category = resolved_category(entry, ctx.theme);
        let colored_label = crate::colors::paint(&label, &category, ctx.theme, ctx.color_enabled);

        let git_col = match ctx.git_status_glyph(&entry.path) {
            Some(g) => format!("{} ", g),
            None if ctx.cli.git_status => "  ".to_string(),
            None => String::new(),
        };

        crate::oprintln!(
            "{perms}  {user:>uw$}  {group:>gw$}  {size:>sw$}  {date}  {git_col}{icon}{label}",
            perms = perms,
            user = user,
            uw = user_w,
            group = group,
            gw = group_w,
            size = size,
            sw = size_width,
            date = date,
            git_col = git_col,
            icon = icon,
            label = colored_label,
        );
    }
}
