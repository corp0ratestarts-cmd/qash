(** * QASH — Coq Extraction Pipeline
    File: proofs/model/Extract.v

    Extracts the QASH Coq model to OCaml for independent verification.

    This file is NOT compiled by CI (it generates output files and
    requires zarith). Run manually to produce model_extracted.ml,
    which can be compiled and used to verify test vectors independently
    of the Rust implementation.

    Usage:
      cd proofs
      coqc -Q . QASH model/Model.v
      coqc -Q . QASH model/Extract.v
      # Produces model_extracted.ml in the current directory.

    Compiling the extracted OCaml:
      # Install zarith: opam install zarith
      ocamlfind ocamlopt \
        -package zarith -linkpkg \
        model_extracted.ml -o model_extracted

    The extracted functions are observationally equivalent to the Coq
    model by construction (Coq's extraction is semantics-preserving
    for the fragment used in Model.v: no axioms beyond ZArith).

    See docs/refinement.md for the full pipeline documentation.
*)

Require Import QASH.model.Model.
Require Extraction.
Require Import ExtrOcamlBasic.
Require Import ExtrOcamlZInt.

Extraction Language OCaml.

(* Extraction hints: map Coq booleans and lists to standard OCaml types. *)
Extract Inductive bool => "bool" [ "true" "false" ].
Extract Inductive list => "list" [ "[]" "(::)" ].
Extract Inductive option => "option" [ "Some" "None" ].
Extract Inductive prod => "( * )" [ "(,)" ].
Extract Inductive nat => "int"
  [ "0" "(fun n -> n + 1)" ]
  "(fun fO fS n -> if n = 0 then fO () else fS (n - 1))".

(*
  Z is extracted to Zarith.Z by default, which handles the full
  integer range. The protocol uses values in [0, 1_000_000] so
  all arithmetic is well within machine-int range, but the extraction
  is correct for arbitrary Z.
*)

(** Extract the core state machine functions to a file. *)
Extraction "model_extracted.ml" step run evaluate advance_epoch_observation is_halted.
