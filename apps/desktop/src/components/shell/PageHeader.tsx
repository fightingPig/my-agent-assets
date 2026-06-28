import type { PageMetadata } from "../../app/pages";

type PageHeaderProps = {
  page: PageMetadata;
};

export function PageHeader({ page }: PageHeaderProps) {
  return (
    <div className="page-header">
      <div className="page-heading">
        <h1>{page.title}</h1>
        <p className="page-subtitle">{page.subtitle}</p>
      </div>
    </div>
  );
}
