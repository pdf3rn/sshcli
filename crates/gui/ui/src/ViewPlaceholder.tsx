type Props = {
  title: string;
  description: string;
};

export default function ViewPlaceholder({ title, description }: Props) {
  return (
    <section className="placeholder-view" aria-labelledby="placeholder-title">
      <h2 id="placeholder-title">{title}</h2>
      <p>{description}</p>
      <p className="muted small">Esta vista llega en la siguiente fase del rediseño.</p>
    </section>
  );
}
