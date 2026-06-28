import { Link } from "@tanstack/react-router";

export function Header() {
  return (
    <header className="py-4 flex gap-2 text-lg justify-between">
      <section>logo</section>
      <section>
        <ul className="flex gap-4 uppercase">
          <li>
            <Link
              to="/"
              activeProps={{
                className: "font-bold",
              }}
              activeOptions={{ exact: true }}
            >
              Home
            </Link>
          </li>
          <li>
            <Link
              preload={false}
              to="/posts"
              activeProps={{
                className: "font-bold",
              }}
            >
              Blog
            </Link>
          </li>
        </ul>
      </section>
    </header>
  );
}
