import { createFileRoute } from "@tanstack/react-router";
import { createServerFn } from "@tanstack/react-start";
import { env } from "cloudflare:workers";

export const Route = createFileRoute("/")({
  loader: () => getData(),
  component: Home,
});

const getData = createServerFn().handler(() => {
  return {
    message: `Running in ${navigator.userAgent}`,
    myVar: env.MY_VAR,
  };
});

function Home() {
  const data = Route.useLoaderData();

  return (
    <div className="py-8 space-y-8">
      <section className="space-y-4">
        <h3 className="text-5xl font-bold text-primary">
          Hey, I&apos;m Christiano!
        </h3>
        <p className="text-lg">
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed in
          malesuada tortor, sed porttitor metus. Nunc mollis et sem eu
          sollicitudin. Curabitur consequat vulputate sagittis. Curabitur
          sodales at felis at pulvinar
        </p>
        <p className="text-lg">
          Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed in
          malesuada tortor, sed porttitor metus. Nunc mollis et sem eu
          sollicitudin.
        </p>
      </section>

      <section>
        <h4 className="text-2xl font-bold text-primary">Blog</h4>

        <p>This Is The Title of a Blog Post</p>
      </section>
      {/* <p>{data.message}</p>
      <p>{data.myVar}</p> */}
    </div>
  );
}
