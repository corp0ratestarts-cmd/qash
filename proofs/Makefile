COQC ?= coqc
COQDEP ?= coqdep

VFILES := util/list_inj.v contractivity/lyapunov_stability.v
VOFILES := $(VFILES:.v=.vo)

all: no-admitted $(VOFILES)

.depend: $(VFILES)
	$(COQDEP) -Q . QASH $(VFILES) > .depend

-include .depend

%.vo: %.v
	$(COQC) -Q . QASH $<

no-admitted:
	@if grep -R --include='*.v' -n 'Admitted\.' .; then \
		echo 'Admitted proofs are forbidden in checked proof files' >&2; \
		exit 1; \
	fi

clean:
	rm -f $(VOFILES) $(VFILES:.v=.vos) $(VFILES:.v=.vok) $(VFILES:.v=.glob) .depend

.PHONY: all clean no-admitted
