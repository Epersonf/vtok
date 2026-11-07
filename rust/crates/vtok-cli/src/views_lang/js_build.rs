//! next build, ng build and prisma: outcome, errors with locations, a few key stats.
use super::util::*;

pub fn next(lines: &[String]) -> Option<String> {
    if !has(lines, r"Next\.js|Compiled|Failed to compile|Route \(") { return None; }
    let routes = lines.iter().filter(|l| re(r"^[┌├└] [ƒ○●λ◐] ").is_match(l)).count();
    let failed = has(lines, r"Failed to compile|Build error occurred|Type error:");
    let mut out = grep(lines, r"Next\.js \d");
    if failed {
        let start = |l: &str| l.contains("Failed to compile") || l.contains("Build error occurred");
        let drop = |l: &str| re(r"^\s*(\d+ \||\|)").is_match(l) || l.trim_start().starts_with("> ");
        out.extend(blocks(lines, &start, &|_| false, 25, &drop));
    } else {
        out.extend(grep(lines, r"Compiled successfully|Compiled with warnings"));
        out.extend(cap(collapse(grep(lines, r"Warning:")), 8));
    }
    if routes > 0 { out.push(format!("{routes} routes")); }
    out.extend(grep(lines, r"First Load JS shared by all"));
    finish(out)
}

pub fn ng(lines: &[String]) -> Option<String> {
    if !has(lines, r"Initial total|bundle generation|Build at:|\[ERROR\]|Error:|Application bundle") { return None; }
    let mut out = vec![];
    let errs = re(r"\[ERROR\]|^Error: |error TS\d+|✘");
    let mut take_loc = false;
    for l in lines {
        if errs.is_match(l) { out.push(short(l.trim(), 200)); take_loc = true; }
        else if take_loc && re(r"^\s+\S+:\d+:\d+:?\s*$").is_match(l) { out.push(format!("  {}", l.trim())); take_loc = false; }
        else if !l.trim().is_empty() { take_loc = false; }
    }
    let warns = grep(lines, r"\[WARNING\]").len();
    if warns > 0 { out.push(format!("{warns} warning(s)")); }
    out.extend(grep(lines, r"Initial total|Application bundle generation|Output location|Build at:|bundle generation (complete|failed)"));
    finish(out)
}

pub fn prisma(lines: &[String]) -> Option<String> {
    if !has(lines, r"(?i)prisma|migration") { return None; }
    let keep = r"Applying migration|migration\(s\)|^\s*└─ \d{6}|in sync|Generated Prisma|Error|error:|\bP\d{4}\b|No pending|drift|Datasource|Already in sync|reset|Database schema|pushed|seed|Warning|warn";
    let mut out = grep(lines, keep);
    out.retain(|l| !l.contains("Start using") && !l.contains("import { PrismaClient"));
    finish(cap(collapse(out), 30))
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::util::t::*;

    #[test]
    fn next_ok_and_failed() {
        let ok = "   ▲ Next.js 14.1.0\n   Creating an optimized production build ...\n ✓ Compiled successfully\n   Linting and checking validity of types ...\n   Collecting page data ...\n ✓ Generating static pages (5/5)\n   Finalizing page optimization ...\n\nRoute (app)                              Size     First Load JS\n┌ ○ /                                    5.2 kB          90 kB\n├ ○ /about                               1 kB            85 kB\n└ ƒ /api/x                               0 B               0 B\n+ First Load JS shared by all            85 kB\n  ├ chunks/23.js                         30 kB\n  └ other shared chunks (total)          2 kB\n\n○  (Static)  prerendered as static content\n";
        let l = lines(ok);
        let o = small(next(&l), &l);
        assert!(o.contains("Next.js 14.1.0") && o.contains("Compiled successfully") && o.contains("3 routes") && o.contains("shared by all"));
        assert!(!o.contains("Collecting page data"));
        let bad = lines("   ▲ Next.js 14.1.0\n   Creating an optimized production build ...\nFailed to compile.\n\n./src/app/page.tsx:5:9\nType error: Type 'number' is not assignable to type 'string'.\n\n  3 | export default function P() {\n> 5 |   const a: string = 1;\n    |         ^\n\n");
        let o = next(&bad).unwrap();
        assert!(o.contains("Failed to compile") && o.contains("./src/app/page.tsx:5:9") && o.contains("Type error") && !o.contains("const a"));
    }

    #[test]
    fn ng_errors_with_location() {
        let s = "Initial chunk files | Names | Raw size\nmain.js | main | 1.2 MB |\npolyfills.js | polyfills | 90 kB |\nstyles.css | styles | 10 kB |\n\n                    | Initial total | 1.3 MB |\n\n✘ [ERROR] TS2322: Type 'string' is not assignable to type 'number'. [plugin angular-compiler]\n\n    src/app/a.component.ts:12:4:\n      12 │   x: number = 'a';\n         ╵   ~\n\n▲ [WARNING] Unused import [plugin x]\n\nApplication bundle generation failed. [1.2 seconds]\n";
        let l = lines(s);
        let o = small(ng(&l), &l);
        assert!(o.contains("TS2322") && o.contains("src/app/a.component.ts:12:4:") && o.contains("1 warning(s)") && o.contains("generation failed"));
        assert!(!o.contains("polyfills") && !o.contains("x: number"));
    }

    #[test]
    fn prisma_drops_banner() {
        let s = "Environment variables loaded from .env\nPrisma schema loaded from prisma/schema.prisma\nDatasource \"db\": PostgreSQL database \"app\", schema \"public\" at \"localhost:5432\"\n\nApplying migration `20240101_init`\n\nThe following migration(s) have been applied:\n\nmigrations/\n  └─ 20240101120000_init/\n    └─ migration.sql\n\nYour database is now in sync with your schema.\n\n✔ Generated Prisma Client (v5.9.0) to ./node_modules/@prisma/client in 120ms\n\nStart using Prisma Client in Node.js (See: https://pris.ly/d/client)\n```\nimport { PrismaClient } from '@prisma/client'\nconst prisma = new PrismaClient()\n```\n";
        let l = lines(s);
        let o = small(prisma(&l), &l);
        assert!(o.contains("Applying migration") && o.contains("20240101120000_init") && o.contains("in sync") && o.contains("Generated Prisma Client"));
        assert!(!o.contains("new PrismaClient") && !o.contains("Start using"));
    }
}
