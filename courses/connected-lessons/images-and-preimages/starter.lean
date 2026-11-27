theorem image_preimage {A B : Type} (f : A → B) (S : A → Prop) (T : B → Prop) :
    (∀ b, (∃ a, S a ∧ f a = b) → T b) ↔ (∀ a, S a → T (f a)) := by
  sorry
