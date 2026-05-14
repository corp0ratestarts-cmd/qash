COQC ?= coqc
COQDEP ?= coqdep
VFILES := util/list_inj.v \
          concat_injective.v lyapunov_decrease.v \
          contractivity/lyapunov_stability.v \
          cascade/cascade_health_bounded.v \
          cascade/cascade_collision_resistance.v \
          cascade/cascade_determinism.v
VOFILES := $(VFILES:.v=.vo)

all: .depend $(VOFILES)
.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend
-include .depend
%.vo: %.v
	$(COQC) -Q . QASH $<
clean:
	rm -f *.vo *.vos *.vok *.glob .depend
.PHONY: all clean
