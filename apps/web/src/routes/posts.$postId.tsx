import { Link, createFileRoute } from "@tanstack/react-router";
import { useMemo } from "react";
import { NotFound } from "~/components/NotFound";
import { PostErrorComponent } from "~/components/PostError";
import { fetchPost } from "~/core/functions/posts";
import { getMDXComponent } from "mdx-bundler/client";

export const Route = createFileRoute("/posts/$postId")({
  loader: ({ params: { postId } }) => fetchPost({ data: postId }),
  errorComponent: PostErrorComponent,
  component: PostComponent,
  notFoundComponent: () => {
    return <NotFound>Post not found</NotFound>;
  },
});

function PostComponent() {
  const post = Route.useLoaderData();

  const Component = useMemo(() => getMDXComponent(post.code), [post.code]);

  return (
    <div className="space-y-2">
      <h4 className="text-xl font-bold underline">{post.title}</h4>
      <div className="text-sm">{post.title}</div>
      <Component />
      <Link
        to="/posts/$postId/deep"
        params={{
          postId: String(post.id),
        }}
        activeProps={{ className: "text-black font-bold" }}
        className="inline-block py-1 text-blue-800 hover:text-blue-600"
      >
        Deep View
      </Link>
    </div>
  );
}
