COQC ?= coqc
COQDEP ?= coqdep
VFILES := util/list_inj.v \
          concat_injective.v lyapunov_decrease.v \
          contractivity/lyapunov_stability.v \
          cascade/cascade_health_bounded.v \
          cascade/cascade_collision_resistance.v \
          cascade/cascade_determinism.v \
          blinding/blinding_non_interference.v
VOFILES := $(VFILES:.v=.vo)

# STRICT_PROOFS: files in the no-Admitted gate.
# This list is APPEND-ONLY — CI enforces it never shrinks across commits.
# To add a proof: add it here AND ensure it compiles without Admitted.
STRICT_PROOFS := \
    contractivity/lyapunov_stability.v \
    cascade/cascade_health_bounded.v

all: .depend $(VOFILES)
.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend
-include .depend
%.vo: %.v
	$(COQC) -Q . QASH $<
clean:
	rm -f *.vo *.vos *.vok *.glob .depend
.PHONY: all clean
